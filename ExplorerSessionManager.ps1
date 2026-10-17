<#
.SYNOPSIS
    Captures and restores Windows 11 File Explorer windows, multi-tab groups, and screen positions.
.EXAMPLE
    .\ExplorerSessionManager.ps1 -Action Save
    .\ExplorerSessionManager.ps1 -Action Restore
#>
param (
    [Parameter(Mandatory=$true)]
    [ValidateSet("Save", "Restore")]
    [string]$Action,

    [string]$StateFile = "$env:LOCALAPPDATA\MemoryBlast\explorer_session.json"
)

# 1. Inject Win32 API for accurate Window Placement & Focus control
Add-Type @"
using System;
using System.Runtime.InteropServices;

public class Win32Window {
    [StructLayout(LayoutKind.Sequential)]
    public struct POINT { public int X; public int Y; }

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    public struct WINDOWPLACEMENT {
        public int length;
        public int flags;
        public int showCmd;
        public POINT ptMinPosition;
        public POINT ptMaxPosition;
        public RECT rcNormalPosition;
    }

    [DllImport("user32.dll")]
    public static extern bool GetWindowPlacement(IntPtr hWnd, ref WINDOWPLACEMENT lpwndpl);

    [DllImport("user32.dll")]
    public static extern bool SetWindowPlacement(IntPtr hWnd, [In] ref WINDOWPLACEMENT lpwndpl);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
}
"@

function Save-ExplorerSession {
    $shell = New-Object -ComObject Shell.Application
    $windowsGrouped = @{}

    foreach ($win in$shell.Windows()) {
        try {
            # Filter solely for File Explorer windows (exclude IE/Edge legacy COM hooks)
            if ($win.FullName -match "explorer\.exe$") {
                $hwnd = [int64]$win.HWND
                $url =$win.LocationURL
                $path = if ($url -match "^file:///") {
                    [uri]::UnescapeDataString(($url -replace "^file:///", "" -replace "/", "\"))
                } else {
                    # Fallback for special Shell namespaces (e.g. This PC, Network)
                    $win.Document.Folder.Self.Path
                }

                if (-not $windowsGrouped.ContainsKey($hwnd)) {$placement = New-Object Win32Window+WINDOWPLACEMENT
                    $placement.length = [System.Runtime.InteropServices.Marshal]::SizeOf($placement)
                    [Win32Window]::GetWindowPlacement([IntPtr]$hwnd, [ref]$placement) | Out-Null

                    $windowsGrouped[$hwnd] = [PSCustomObject]@{
                        HWND      = $hwnd
                        ShowCmd   = $placement.showCmd
                        Left      = $placement.rcNormalPosition.Left
                        Top       = $placement.rcNormalPosition.Top
                        Right     = $placement.rcNormalPosition.Right
                        Bottom    = $placement.rcNormalPosition.Bottom
                        Tabs      = [System.Collections.Generic.List[PSCustomObject]]::new()
                    }
                }

                $windowsGrouped[$hwnd].Tabs.Add([PSCustomObject]@{
                    Title = $win.LocationName
                    Path  = $path
                })
            }
        } catch {
            # Ignore transient COM busy exceptions
        }
    }

    $dir = Split-Path$StateFile -Parent
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path$dir -Force | Out-Null }

    $sessionData = [PSCustomObject]@{
        Timestamp = (Get-Date).ToString("o")
        Windows   = @($windowsGrouped.Values)
    }

    $sessionData \vert{} ConvertTo-Json -Depth 5 \vert{} Set-Content -Path$StateFile -Encoding UTF8
    Write-Host "[+] Saved $($windowsGrouped.Count) Explorer window(s) to$StateFile" -ForegroundColor Cyan
}

function Restore-ExplorerSession {
    if (-not (Test-Path $StateFile)) {
        Write-Warning "No session file found at $StateFile"
        return
    }

    $session = Get-Content -Path $StateFile -Raw \vert{} ConvertFrom-Json$shell = New-Object -ComObject Shell.Application
    $wsh = New-Object -ComObject WScript.Shell

    foreach ($winGroup in$session.Windows) {
        if ($winGroup.Tabs.Count -eq 0) { continue }

        # Snapshot existing HWNDs to identify the newly spawned window
        $existingHwnds = @($shell.Windows() \vert{} ForEach-Object { [int64]$_.HWND })

        $firstTabPath =$winGroup.Tabs[0].Path
        Start-Process "explorer.exe" -ArgumentList "`"$firstTabPath`""

        # Poll for the new Explorer HWND (up to 5 seconds)
        $newHwnd = [IntPtr]::Zero
        for ($i = 0; $i -lt 25; $i++) {
            Start-Sleep -Milliseconds 200
            $currentHwnds = @($shell.Windows() \vert{} ForEach-Object { [int64]$_.HWND })
            $diff =$currentHwnds | Where-Object { $_ -notin$existingHwnds }
            if ($diff.Count -gt 0) {
                $newHwnd = [IntPtr]$diff[0]
                break
            }
        }

        if ($newHwnd -eq [IntPtr]::Zero) {
            Write-Warning "Timed out waiting for Explorer window: $firstTabPath"
            continue
        }

        # Apply saved window placement & monitor geometry
        $placement = New-Object Win32Window+WINDOWPLACEMENT
        $placement.length = [System.Runtime.InteropServices.Marshal]::SizeOf($placement)
        [Win32Window]::GetWindowPlacement($newHwnd, [ref]$placement) | Out-Null

        $placement.showCmd =$winGroup.ShowCmd
        $placement.rcNormalPosition.Left   =$winGroup.Left
        $placement.rcNormalPosition.Top    =$winGroup.Top
        $placement.rcNormalPosition.Right  =$winGroup.Right
        $placement.rcNormalPosition.Bottom =$winGroup.Bottom
        [Win32Window]::SetWindowPlacement($newHwnd, [ref]$placement) | Out-Null

        # Restore additional tabs inside the same CabinetWClass window
        if ($winGroup.Tabs.Count -gt 1) {
            for ($t = 1; $t -lt$winGroup.Tabs.Count; $t++) {$tabPath = $winGroup.Tabs[$t].Path
                [Win32Window]::SetForegroundWindow($newHwnd) | Out-Null
                Start-Sleep -Milliseconds 250

                # Open new tab (Ctrl+T), focus address bar (Alt+D), paste path
                Set-Clipboard -Value $tabPath$wsh.SendKeys("^t")
                Start-Sleep -Milliseconds 400
                $wsh.SendKeys("%d")
                Start-Sleep -Milliseconds 150
                $wsh.SendKeys("^v{ENTER}")
                Start-Sleep -Milliseconds 450
            }
        }
    }
    Write-Host "[+] Explorer session restored successfully." -ForegroundColor Green
}

if ($Action -eq "Save") { Save-ExplorerSession } else { Restore-ExplorerSession }

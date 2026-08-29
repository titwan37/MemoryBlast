@echo off
echo ========================================================
echo  Initializing Git Repository for MemoryBlast
echo ========================================================

@REM 1. Initialize local repository
git init

@REM 2. Ensure default branch is main
git branch -M main

@REM 3. Stage all files (respects .gitignore: ignores /target/, etc.)
git add .

@REM 4. Verify staged files
echo.
echo Staged files:
git status -s

@REM 5. Create initial commit
echo.
echo Creating initial commit...
git commit -m "Initial commit: MemoryBlast - High-Performance Windows Process Manager & Chromium Inspector"

@REM 6. Configure remote repository
git remote remove origin 2>nul
git remote add origin https://github.com/titwan37/MemoryBlast.git
echo Remote origin set to: https://github.com/titwan37/MemoryBlast.git

@REM 7. Push to GitHub
echo.
echo Pushing to GitHub (main branch)...
git push -u origin main

echo.
echo ========================================================
echo  Repository successfully published to GitHub!
echo ========================================================
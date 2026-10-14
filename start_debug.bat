@echo off
title Parallel Script Launcher

:: Launch MemoryBlast in a separate window
start "MemoryBlast" powershell.exe -NoExit -Command "& 'C:\Dev\MemoryBlast\target\debug\memoryblast.exe' --cli"

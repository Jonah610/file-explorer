
$ErrorActionPreference = "Stop"

$Repo = "Jonah610/file-explorer"

$Architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture

switch ($Architecture.ToString()) {
    "X64" {
        $Target = "x86_64-pc-windows-msvc"
    }
    "Arm64" {
        $Target = "aarch64-pc-windows-msvc"
    }
    default {
        throw "Unsupported Windows architecture: $Architecture"
    }
}

$Url = "https://github.com/$Repo/releases/latest/download/fe-$Target.exe"

$InstallDir = "$HOME\.local\bin"
$Destination = "$InstallDir\fe.exe"

Write-Host "Downloading fe for $Target..."

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

Invoke-WebRequest -Uri $Url -OutFile $Destination

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")

if (($UserPath -split ";") -notcontains $InstallDir) {
    $NewPath = if ($UserPath) {
        "$UserPath;$InstallDir"
    } else {
        $InstallDir
    }

    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
}

Write-Host ""
Write-Host "Successfully installed fe!"
Write-Host "Restart your terminal and run: fe"

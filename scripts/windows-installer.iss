; The Windows download: one setup program that puts Chess and its Stockfish side by side in the
; user's programs folder, with a Start menu entry and an uninstaller. Built by the release
; workflow from dist/Chess once that folder holds Chess.exe, stockfish.exe and Stockfish's licence:
;   ISCC.exe /DAppVersion=0.1.0 scripts\windows-installer.iss

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
; Never change it: Windows knows an installed Chess by it, to upgrade or remove it.
AppId={{3C98DC97-BE9E-46B2-8B6B-8D9FD0DA980D}
AppName=Chess
AppVersion={#AppVersion}
AppVerName=Chess {#AppVersion}
; For this user only, so it installs without asking for an administrator.
PrivilegesRequired=lowest
DefaultDirName={autopf}\Chess
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\Chess.exe
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=Chess-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
; Chess must be closed to be replaced.
CloseApplications=yes

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\dist\Chess\Chess.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\Chess\stockfish.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\Chess\Stockfish-COPYING.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Chess"; Filename: "{app}\Chess.exe"
Name: "{autodesktop}\Chess"; Filename: "{app}\Chess.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\Chess.exe"; Description: "{cm:LaunchProgram,Chess}"; Flags: nowait postinstall skipifsilent

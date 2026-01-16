#define MyAppName "Hackmaster Sim"
#define MyAppVersion "0.1.0.1"
#define MyAppExeName "sim_gui.exe"
#define MyCertFile SourcePath + "\..\secrets\codesign\mygame-dev.cer"
#define DefaultBinDir "target\\release"

#ifndef BuildTarget
#define BuildTarget ""
#endif

#if BuildTarget != ""
#define BinDir "target\\" + BuildTarget + "\\release"
#else
#define BinDir DefaultBinDir
#endif

#ifexist MyCertFile
#define IncludeCert
#endif

[Setup]
AppId={{E2F6C5E8-1E79-4A7D-9B13-8F1C0C9B2E10}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
DefaultDirName={pf}\HackmasterSim
DefaultGroupName=HackmasterSim
OutputBaseFilename=HackmasterSimSetup
OutputDir={#SourcePath}\dist
SetupIconFile={#SourcePath}\..\assets\icon_sim_gui.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64

[Dirs]
Name: "{app}\data"; Permissions: users-modify

[Files]
Source: "{#SourcePath}\..\{#BinDir}\sim_gui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourcePath}\..\{#BinDir}\autobattler.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourcePath}\..\{#BinDir}\sim_cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourcePath}\..\{#BinDir}\hackmaster_sim.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourcePath}\..\data\*"; DestDir: "{app}\data"; Flags: recursesubdirs createallsubdirs
#ifdef IncludeCert
Source: "{#MyCertFile}"; DestDir: "{tmp}"; Flags: deleteafterinstall
#endif

#ifdef IncludeCert
[Tasks]
Name: "trustcert"; Description: "Trust Hackmaster Sim developer certificate (Current User)"; Flags: unchecked
#endif

[Icons]
Name: "{group}\Hackmaster Sim"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"

#ifdef IncludeCert
[Run]
Filename: "certutil.exe"; Parameters: "-user -addstore ""Root"" ""{tmp}\mygame-dev.cer"""; StatusMsg: "Installing developer certificate..."; Flags: runhidden; Tasks: trustcert
Filename: "certutil.exe"; Parameters: "-user -addstore ""TrustedPublisher"" ""{tmp}\mygame-dev.cer"""; StatusMsg: "Installing developer certificate..."; Flags: runhidden; Tasks: trustcert
#endif

; Aetheris — Inno Setup installer script
;
; Preprocessor defines passed by CI:
;   AppVersion    — e.g. "0.1.0"
;   SourceDir     — path to the bundled Windows folder
;   OutputDir     — where to write the installer exe
;   TargetArch    — "x64" or "arm64"
;   SetupIconFile — optional .ico path

#ifndef AppVersion
  #define AppVersion "1.4.1"
#endif
#ifndef SourceDir
  #define SourceDir "..\..\target\windows\aetheris"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist"
#endif
#ifndef TargetArch
  #define TargetArch "x64"
#endif

[Setup]
AppName=Aetheris
AppVersion={#AppVersion}
AppVerName=Aetheris
AppId={{7F9E4C2A-5425-4A7E-8A0B-CA8B395E7C21}
VersionInfoVersion={#AppVersion}
AppPublisher=LuminusOS
AppPublisherURL=https://github.com/luminusOS/aetheris
AppSupportURL=https://github.com/luminusOS/aetheris/issues
AppUpdatesURL=https://github.com/luminusOS/aetheris/releases
DefaultDirName={localappdata}\Programs\Aetheris
DefaultGroupName=Aetheris
UninstallDisplayIcon={app}\bin\aetheris.exe
OutputDir={#OutputDir}
OutputBaseFilename=aetheris-setup
Compression=lzma2/ultra64
SolidCompression=yes
LicenseFile=..\..\LICENSE
WizardStyle=modern
PrivilegesRequired=lowest
DisableDirPage=auto
DisableProgramGroupPage=auto
CloseApplications=yes
CloseApplicationsFilter=aetheris.exe
SetupLogging=yes
#ifdef SetupIconFile
SetupIconFile={#SetupIconFile}
#endif
#if TargetArch == "arm64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Aetheris"; Filename: "{app}\bin\aetheris.exe"
Name: "{group}\Uninstall Aetheris"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Aetheris"; Filename: "{app}\bin\aetheris.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\bin\aetheris.exe"; Description: "Launch Aetheris"; Flags: nowait postinstall skipifnotsilent

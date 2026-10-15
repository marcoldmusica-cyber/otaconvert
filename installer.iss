; Instalador de Otaconvert (Inno Setup 6, gratuito). Instala solo para el usuario: no pide permisos de administrador.
#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif

[Setup]
AppId={{7C1D3B5E-4A62-4E0B-9C57-0F7A2B6D8E41}
AppName=Otaconvert
AppVersion={#AppVersion}
AppPublisher=Otaconvert
DefaultDirName={autopf}\Otaconvert
DefaultGroupName=Otaconvert
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=dist\release
OutputBaseFilename=Otaconvert-Setup
SetupIconFile=assets\otaconvert.ico
UninstallDisplayIcon={app}\Otaconvert.exe
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "dist\Otaconvert\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{autoprograms}\Otaconvert"; Filename: "{app}\Otaconvert.exe"
Name: "{autodesktop}\Otaconvert"; Filename: "{app}\Otaconvert.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\Otaconvert.exe"; Description: "{cm:LaunchProgram,Otaconvert}"; Flags: nowait postinstall skipifsilent

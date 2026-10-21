; Instalador de Otaconvert V2 (Inno Setup 6, gratuito). Etapa 6 de la Fase 1.
; - Solo para el usuario: no pide permisos de administrador.
; - Convive con la V1 (otro nombre y otro AppId).
; - Asocia los archivos .otashow (doble clic abre el show) solo para este usuario.
; - El desinstalador NUNCA borra los shows ni los ajustes (carpeta de datos
;   %LOCALAPPDATA%\Otaconvert): se pueden volver a usar al reinstalar.
; Lo arma el workflow V2 a partir de la carpeta de la versión portable
; (sin portable.txt). Uso: iscc /DAppVersion=x.y.z /DOrigen=<carpeta> otaconvert-v2.iss
#ifndef AppVersion
  #define AppVersion "0.0.1"
#endif
#ifndef Origen
  #define Origen "..\demo\Otaconvert-V2"
#endif

[Setup]
AppId={{64A53954-3B3A-42BF-B091-4902CF3464BA}
AppName=Otaconvert V2
AppVersion={#AppVersion}
AppVerName=Otaconvert V2 {#AppVersion}
AppPublisher=Otaconvert (proyecto libre, @marcold.ok)
AppPublisherURL=https://github.com/marcoldmusica-cyber/otaconvert
AppSupportURL=https://github.com/marcoldmusica-cyber/otaconvert
DefaultDirName={autopf}\Otaconvert V2
DefaultGroupName=Otaconvert V2
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.14393
OutputDir=salida
OutputBaseFilename=Otaconvert-V2-Instalador-{#AppVersion}
SetupIconFile=..\..\assets\otaconvert.ico
UninstallDisplayIcon={app}\Otaconvert V2.exe
UninstallDisplayName=Otaconvert V2
LicenseFile=..\..\LICENSE
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ChangesAssociations=yes
VersionInfoProductName=Otaconvert
VersionInfoVersion={#AppVersion}
VersionInfoDescription=Instalador de Otaconvert V2

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
spanish.AsociarOtashow=Abrir los archivos .otashow con Otaconvert V2
english.AsociarOtashow=Open .otashow files with Otaconvert V2
spanish.ShowOtaconvert=Show de Otaconvert V2
english.ShowOtaconvert=Otaconvert V2 show
spanish.Manual=Manual de usuario de Otaconvert V2
english.Manual=Otaconvert V2 user manual
spanish.PruebaHardware=Prueba de hardware de Otaconvert V2
english.PruebaHardware=Otaconvert V2 hardware test

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "asociar"; Description: "{cm:AsociarOtashow}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#Origen}\*"; DestDir: "{app}"; Excludes: "portable.txt,datos"; Flags: recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{autoprograms}\Otaconvert V2"; Filename: "{app}\Otaconvert V2.exe"
Name: "{autoprograms}\{cm:Manual}"; Filename: "{app}\Manual de usuario.pdf"
Name: "{autoprograms}\{cm:PruebaHardware}"; Filename: "{app}\Prueba de hardware.exe"
Name: "{autodesktop}\Otaconvert V2"; Filename: "{app}\Otaconvert V2.exe"; Tasks: desktopicon

[Registry]
; Solo para este usuario (HKCU): nada que pida administrador.
Root: HKCU; Subkey: "Software\Classes\.otashow"; ValueType: string; ValueName: ""; ValueData: "Otaconvert2.Show"; Flags: uninsdeletevalue; Tasks: asociar
Root: HKCU; Subkey: "Software\Classes\Otaconvert2.Show"; ValueType: string; ValueName: ""; ValueData: "{cm:ShowOtaconvert}"; Flags: uninsdeletekey; Tasks: asociar
Root: HKCU; Subkey: "Software\Classes\Otaconvert2.Show\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\Otaconvert V2.exe,0"; Tasks: asociar
Root: HKCU; Subkey: "Software\Classes\Otaconvert2.Show\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\Otaconvert V2.exe"" ""%1"""; Tasks: asociar

[Run]
Filename: "{app}\Otaconvert V2.exe"; Description: "{cm:LaunchProgram,Otaconvert V2}"; Flags: nowait postinstall skipifsilent

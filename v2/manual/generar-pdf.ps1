# Genera manual-usuario.pdf a partir de manual-usuario.html con Microsoft Edge
# (viene con Windows 10 y 11). Ejecutar después de cada cambio en el manual.
# Uso: clic derecho > "Ejecutar con PowerShell", o: powershell -File generar-pdf.ps1

$ErrorActionPreference = 'Stop'
$aqui = Split-Path -Parent $MyInvocation.MyCommand.Path
$html = Join-Path $aqui 'manual-usuario.html'
$pdf  = Join-Path $aqui 'manual-usuario.pdf'

$edge = @(
  "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
  "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { throw 'No se encontró Microsoft Edge.' }

# El manual se imprime siempre con el tema oscuro normal del programa.
$url = ([System.Uri]$html).AbsoluteUri
if (Test-Path $pdf) { Remove-Item $pdf }
# Edge escribe avisos sin importancia en la consola: se ignoran.
$argumentos = @('--headless=new', '--disable-gpu', '--no-pdf-header-footer', "--print-to-pdf=`"$pdf`"", $url)
Start-Process -FilePath $edge -ArgumentList $argumentos -Wait -WindowStyle Hidden -RedirectStandardError (Join-Path $env:TEMP 'otaconvert-edge.log')
if (-not (Test-Path $pdf)) { throw 'No se generó el PDF.' }
Write-Output "PDF generado: $pdf"

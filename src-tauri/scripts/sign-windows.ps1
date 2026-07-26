param(
    [Parameter(Mandatory = $true)]
    [string]$File
)

$thumbprint = $env:WINDOWS_CERTIFICATE_THUMBPRINT
if ([string]::IsNullOrWhiteSpace($thumbprint)) {
    Write-Host "WINDOWS_CERTIFICATE_THUMBPRINT is not configured; leaving $File unsigned."
    exit 0
}

$signTool = Get-Command "signtool.exe" -ErrorAction Stop
$arguments = @(
    "sign",
    "/sha1", $thumbprint,
    "/fd", "SHA256"
)

$timestampUrl = $env:WINDOWS_TIMESTAMP_URL
if (-not [string]::IsNullOrWhiteSpace($timestampUrl)) {
    $arguments += @("/tr", $timestampUrl, "/td", "SHA256")
}

$arguments += $File
& $signTool.Source @arguments
exit $LASTEXITCODE

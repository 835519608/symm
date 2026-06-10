param(
    [Parameter(Mandatory = $true, ValueFromRemainingArguments = $true)]
    [string[]] $Path
)

$ErrorActionPreference = "Stop"

$forbiddenImports = @(
    "VCRUNTIME140.dll",
    "VCRUNTIME140_1.dll",
    "MSVCP140.dll",
    "api-ms-win-crt-"
)

foreach ($item in $Path) {
    if (-not (Test-Path -LiteralPath $item -PathType Leaf)) {
        throw "找不到待校验文件：$item"
    }

    $resolved = (Resolve-Path -LiteralPath $item).ProviderPath
    $bytes = [System.IO.File]::ReadAllBytes($resolved)
    $text = [System.Text.Encoding]::Latin1.GetString($bytes)
    $found = @($forbiddenImports | Where-Object { $text.Contains($_) })
    if ($found.Count -gt 0) {
        throw "Windows 发布二进制仍依赖动态 VC Runtime：$item -> $($found -join ', ')"
    }

    Write-Host "静态 CRT 校验通过：$item"
}

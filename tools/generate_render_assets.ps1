Add-Type -AssemblyName System.Drawing

$root = Split-Path -Parent $PSScriptRoot
$textureDir = Join-Path $root "assets\textures"
$skyDir = Join-Path $root "assets\skybox"
New-Item -ItemType Directory -Force -Path $skyDir | Out-Null

function Limit($value, $minimum, $maximum) {
    return [Math]::Min($maximum, [Math]::Max($minimum, $value))
}

function New-NormalMap($sourceName, $destinationName, $strength) {
    $source = Join-Path $textureDir $sourceName
    if (-not (Test-Path -LiteralPath $source)) { return }
    $sourceImage = [System.Drawing.Bitmap]::new($source)
    $inputImage = [System.Drawing.Bitmap]::new($sourceImage, 128, 128)
    $sourceImage.Dispose()
    $outputImage = [System.Drawing.Bitmap]::new($inputImage.Width, $inputImage.Height)
    try {
        for ($y = 0; $y -lt $inputImage.Height; $y++) {
            for ($x = 0; $x -lt $inputImage.Width; $x++) {
                $xl = ($x - 1 + $inputImage.Width) % $inputImage.Width
                $xr = ($x + 1) % $inputImage.Width
                $yu = ($y - 1 + $inputImage.Height) % $inputImage.Height
                $yd = ($y + 1) % $inputImage.Height
                $left = $inputImage.GetPixel($xl, $y).GetBrightness()
                $right = $inputImage.GetPixel($xr, $y).GetBrightness()
                $up = $inputImage.GetPixel($x, $yu).GetBrightness()
                $down = $inputImage.GetPixel($x, $yd).GetBrightness()
                $nx = ($left - $right) * $strength
                $ny = ($up - $down) * $strength
                $nz = 1.0
                $length = [Math]::Sqrt($nx*$nx + $ny*$ny + $nz*$nz)
                $r = Limit ([int](($nx/$length*0.5+0.5)*255)) 0 255
                $g = Limit ([int](($ny/$length*0.5+0.5)*255)) 0 255
                $b = Limit ([int](($nz/$length*0.5+0.5)*255)) 0 255
                $outputImage.SetPixel($x, $y, [System.Drawing.Color]::FromArgb($r, $g, $b))
            }
        }
        $outputImage.Save((Join-Path $textureDir $destinationName), [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $inputImage.Dispose()
        $outputImage.Dispose()
    }
}

New-NormalMap "stone.png" "stone_normal.png" 4.0
New-NormalMap "water.png" "water_normal.png" 2.2
New-NormalMap "netherrack.png" "netherrack_normal.png" 3.5
New-NormalMap "obsidian.png" "obsidian_normal.png" 1.8

function Get-SkyDirection($face, $u, $v) {
    switch ($face) {
        "px" { $x=1.0;  $y=-$v; $z=-$u }
        "nx" { $x=-1.0; $y=-$v; $z=$u }
        "py" { $x=$u;   $y=1.0; $z=$v }
        "ny" { $x=$u;   $y=-1.0;$z=-$v }
        "pz" { $x=$u;   $y=-$v; $z=1.0 }
        "nz" { $x=-$u;  $y=-$v; $z=-1.0 }
    }
    $length=[Math]::Sqrt($x*$x+$y*$y+$z*$z)
    return @(($x/$length),($y/$length),($z/$length))
}

$size=128
foreach ($face in @("px","nx","py","ny","pz","nz")) {
    $bitmap=[System.Drawing.Bitmap]::new($size,$size)
    try {
        for($y=0;$y -lt $size;$y++) {
            for($x=0;$x -lt $size;$x++) {
                $u=2.0*$x/($size-1)-1.0; $v=2.0*$y/($size-1)-1.0
                $d=Get-SkyDirection $face $u $v; $dx=$d[0];$dy=$d[1];$dz=$d[2]
                $t=Limit ($dy*0.5+0.5) 0.0 1.0
                $r=0.68*(1-$t)+0.055*$t; $g=0.82*(1-$t)+0.12*$t; $b=1.0*(1-$t)+0.30*$t
                $cloudField=([Math]::Sin($dx*9+$dz*3)+[Math]::Sin($dz*13-$dx*2)*0.55+[Math]::Sin($dx*21+$dz*17)*0.2)/1.75
                $cloud=(Limit (($cloudField-0.28)/0.5) 0.0 1.0)*(1-$t)*0.14
                $sunDot=[Math]::Max(0,$dx*(-0.496)+$dy*0.794+$dz*(-0.353))
                $sun=[Math]::Pow($sunDot,180)*7
                $r=Limit ($r+$cloud+$sun) 0.0 1.0;$g=Limit ($g+$cloud+$sun*0.72) 0.0 1.0;$b=Limit ($b+$cloud+$sun*0.38) 0.0 1.0
                $bitmap.SetPixel($x,$y,[System.Drawing.Color]::FromArgb([int]($r*255),[int]($g*255),[int]($b*255)))
            }
        }
        $bitmap.Save((Join-Path $skyDir ("sky_"+$face+".png")),[System.Drawing.Imaging.ImageFormat]::Png)
    } finally { $bitmap.Dispose() }
}

Write-Host "Normal maps and six-face skybox generated."

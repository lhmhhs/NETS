
# MRPUZZLE-NETS 아이콘 생성: 퍼즐 조각 모양 + 네트워크 노드 (ICO, 16/32/48/256)
Add-Type -AssemblyName System.Drawing

$outPath = Join-Path $PSScriptRoot "..\assets\nets.ico"
New-Item -ItemType Directory -Force -Path (Split-Path $outPath) | Out-Null

function New-PuzzleIcon([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)
    $s = $size / 256.0

    # 배경 그라데이션 (둥근 사각형)
    $bg = New-Object System.Drawing.Drawing2D.GraphicsPath
    $r = 56 * $s
    $bg.AddArc(0, 0, 2 * $r, 2 * $r, 180, 90)
    $bg.AddArc(256 * $s - 2 * $r, 0, 2 * $r, 2 * $r, 270, 90)
    $bg.AddArc(256 * $s - 2 * $r, 256 * $s - 2 * $r, 2 * $r, 2 * $r, 0, 90)
    $bg.AddArc(0, 256 * $s - 2 * $r, 2 * $r, 2 * $r, 90, 90)
    $bg.CloseFigure()
    $bgBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        (New-Object System.Drawing.RectangleF 0, 0, $size, $size),
        [System.Drawing.Color]::FromArgb(255, 60, 140, 255),
        [System.Drawing.Color]::FromArgb(255, 25, 60, 190), 60)
    $g.FillPath($bgBrush, $bg)

    # 퍼즐 조각 (몸통 + 위/오른쪽/아래 볼록 탭, 왼쪽은 오목하게 비움)
    $pieceBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 255, 255, 255))
    $fillA = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(40, 255, 255, 255))
    $body = New-Object System.Drawing.Drawing2D.GraphicsPath
    $body.AddRectangle((New-Object System.Drawing.RectangleF (60 * $s), (66 * $s), (136 * $s), (136 * $s)))
    $g.FillPath($pieceBrush, $body)
    $g.FillEllipse($pieceBrush, (98 * $s), (30 * $s), (60 * $s), (60 * $s))   # 위 탭
    $g.FillEllipse($pieceBrush, (166 * $s), (98 * $s), (60 * $s), (60 * $s))  # 오른쪽 탭
    $g.FillEllipse($pieceBrush, (98 * $s), (166 * $s), (60 * $s), (60 * $s))  # 아래 탭
    # 왼쪽 오목 (배경색으로 덮어 홈 표현)
    $g.FillEllipse($bgBrush, (30 * $s), (98 * $s), (60 * $s), (60 * $s))

    # 네트워크 표시: 노드 3개를 선으로 연결
    $nodes = @(
        @(104, 104),
        @(152, 104),
        @(128, 150)
    )
    $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 60, 140, 255)), (7 * $s)
    for ($i = 0; $i -lt $nodes.Count; $i++) {
        for ($j = $i + 1; $j -lt $nodes.Count; $j++) {
            $g.DrawLine($pen,
                $nodes[$i][0] * $s, $nodes[$i][1] * $s,
                $nodes[$j][0] * $s, $nodes[$j][1] * $s)
        }
    }
    foreach ($n in $nodes) {
        $cx = $n[0] * $s; $cy = $n[1] * $s; $rad = 15 * $s
        $g.FillEllipse($pieceBrush, ($cx - $rad), ($cy - $rad), (2 * $rad), (2 * $rad))
        $ir = 7 * $s
        $inner = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 60, 140, 255))
        $g.FillEllipse($inner, ($cx - $ir), ($cy - $ir), (2 * $ir), (2 * $ir))
    }

    $g.Dispose()
    return $bmp
}

$sizes = @(16, 32, 48, 256)
$pngs = @()
$master = New-PuzzleIcon 256
foreach ($sz in $sizes) {
    if ($sz -eq 256) {
        $img = $master
    } else {
        $img = New-Object System.Drawing.Bitmap $sz, $sz
        $gg = [System.Drawing.Graphics]::FromImage($img)
        $gg.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $gg.DrawImage($master, 0, 0, $sz, $sz)
        $gg.Dispose()
    }
    $ms = New-Object System.IO.MemoryStream
    $img.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $pngs += ,@($sz, $ms.ToArray())
}

# ICO 파일 쓰기 (PNG 임베드 방식)
$fs = [System.IO.File]::Create($outPath)
$bw = New-Object System.IO.BinaryWriter $fs
$bw.Write([uint16]0)
$bw.Write([uint16]1)
$bw.Write([uint16]$pngs.Count)
$offset = 6 + 16 * $pngs.Count
foreach ($p in $pngs) {
    $sz = [int]$p[0]
    $data = [byte[]]$p[1]
    $w = if ($sz -ge 256) { 0 } else { $sz }
    $bw.Write([byte]$w)
    $bw.Write([byte]$w)
    $bw.Write([byte]0)
    $bw.Write([byte]0)
    $bw.Write([uint16]1)
    $bw.Write([uint16]32)
    $bw.Write([uint32]$data.Length)
    $bw.Write([uint32]$offset)
    $offset += $data.Length
}
foreach ($p in $pngs) { $bw.Write([byte[]]$p[1]) }
$bw.Close()
$fs.Close()

Write-Output "wrote $outPath ($((Get-Item $outPath).Length) bytes)"

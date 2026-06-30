param([string]$WindowTitle = "ScreenAnimation")

# 1. Assemblies laden (Muss am Anfang stehen)
Add-Type -AssemblyName System.Windows.Forms, System.Drawing

# --- KONFIGURATION ---
$cfg = @{
    PulseFrames    = 32
    PulseIntensity = 0.05
    PulseSpeedMs   = 10
    WooshFrames    = 90
    WooshSpeedMs   = 10
    WooshEasing    = 3
    CsvPath        = "C:\Scripte\MonitorSetupSwitch\monitorsinfo.csv"
    Sound1         = "C:\Scripte\MonitorSetupSwitch\Sounds\sound5.wav"
    Sound2         = "C:\Scripte\MonitorSetupSwitch\Sounds\switch.wav"
    MaxWaitSeconds = 8
}

# --- NATIVE API EXTENSION ---
$apiCode = @"
using System;
using System.Runtime.InteropServices;
public class WinUtil {
    [DllImport("user32.dll", SetLastError = true)] public static extern bool SystemParametersInfo(uint uiAction, uint uiParam, ref bool pvParam, uint fWinIni);
    [DllImport("user32.dll", SetLastError = true)] public static extern bool SystemParametersInfo(uint uiAction, uint uiParam, IntPtr pvParam, uint fWinIni);
    [DllImport("user32.dll")] public static extern bool LockWindowUpdate(IntPtr hWndLock);
    [DllImport("user32.dll", CharSet = CharSet.Auto)] public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, IntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out IntPtr lpdwResult);

    public const uint SPI_SETANIMATION = 0x0049;
    public const uint SPI_SETCLIENTAREAANIMATION = 0x1043;
    public const uint WM_SETTINGCHANGE = 0x001A;
    public const uint HWND_BROADCAST = 0xffff;

    [StructLayout(LayoutKind.Sequential)]
    public struct ANIMATIONINFO {
        public uint cbSize;
        public int iMinAnimate;
    }

    public static void ToggleVisuals(bool enable) {
        ANIMATIONINFO ai = new ANIMATIONINFO();
        ai.cbSize = (uint)Marshal.SizeOf(ai);
        ai.iMinAnimate = enable ? 1 : 0;
        
        IntPtr ptr = Marshal.AllocHGlobal(Marshal.SizeOf(ai));
        Marshal.StructureToPtr(ai, ptr, false);
        SystemParametersInfo(SPI_SETANIMATION, ai.cbSize, ptr, 0x01 | 0x02);
        Marshal.FreeHGlobal(ptr);

        bool state = enable;
        SystemParametersInfo(SPI_SETCLIENTAREAANIMATION, 0, ref state, 0x01 | 0x02);

        IntPtr res;
        SendMessageTimeout((IntPtr)HWND_BROADCAST, WM_SETTINGCHANGE, IntPtr.Zero, "TrayTab", 0x0002, 1000, out res);
    }
}
"@

if (-not ([System.Management.Automation.PSTypeName]"WinUtil").Type) { Add-Type -TypeDefinition $apiCode }

# --- STATE MANAGEMENT & LIST INITIALIZATION ---
$RegPath = "HKCU:\Control Panel\Desktop\WindowMetrics"
$originalState = (Get-ItemProperty $RegPath).MinAnimate -eq "1"

# Sichere Instanziierung der Listen
$forms = [System.Collections.Generic.List[System.Windows.Forms.Form]]::new()
$bitmaps = [System.Collections.Generic.List[System.Drawing.Bitmap]]::new()
$players = [System.Collections.Generic.List[System.Media.SoundPlayer]]::new()

function Set-VisualSafety($lock) {
    [void][WinUtil]::ToggleVisuals(-not $lock)
    $h = if ($lock) { [IntPtr]1 } else { [IntPtr]0 }
    [void][WinUtil]::LockWindowUpdate($h)
}

try {
    ([System.Diagnostics.Process]::GetCurrentProcess()).PriorityClass = 'High'
    
    # Deaktivierung VOR dem GUI-Aufbau
    Set-VisualSafety $true

    $initialState = $false
    if (Test-Path $cfg.CsvPath) {
        if (Select-String -Path $cfg.CsvPath -Pattern "TCL0000.*Yes" -Quiet -ErrorAction SilentlyContinue) { $initialState = $true }
    }

    foreach ($screen in [System.Windows.Forms.Screen]::AllScreens) {
        $f = [System.Windows.Forms.Form]::new()
        $f.FormBorderStyle = 'None'; $f.StartPosition = 'Manual'
        $f.Location = $screen.Bounds.Location; $f.Size = $screen.Bounds.Size
        $f.BackColor = 'Black'; $f.TopMost = $true
        
        $flags = [System.Reflection.BindingFlags]::Instance -bor [System.Reflection.BindingFlags]::NonPublic
        $f.GetType().GetProperty("DoubleBuffered", $flags).SetValue($f, $true, $null)
        
        $bmp = [System.Drawing.Bitmap]::new($screen.Bounds.Width, $screen.Bounds.Height)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        try { $g.CopyFromScreen($screen.Bounds.Location, [System.Drawing.Point]::Empty, $screen.Bounds.Size) } catch { }
        $g.Dispose() 
        
        $pb = [System.Windows.Forms.PictureBox]::new()
        $pb.Image = $bmp; $pb.SizeMode = 'StretchImage'; $pb.Size = $screen.Bounds.Size
        $f.Controls.Add($pb)
        
        $bitmaps.Add($bmp); $forms.Add($f)
        $f.Show()
    }

    # ANIMATION LOGIC
    if (Test-Path $cfg.Sound1) { $p = [System.Media.SoundPlayer]::new($cfg.Sound1); $p.Play(); $players.Add($p) }
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    for ($i=0; $i -le $cfg.PulseFrames; $i++) {
        $scale = 1 - ($cfg.PulseIntensity * [math]::Sin(($i/$cfg.PulseFrames) * [math]::PI))
        foreach ($form in $forms) {
            $nW = [int]($form.Width * $scale); $nH = [int]($form.Height * $scale)
            $form.Controls[0].SetBounds([int](($form.Width - $nW)/2), [int](($form.Height - $nH)/2), $nW, $nH, 15)
        }
        [System.Windows.Forms.Application]::DoEvents()
        while ($sw.ElapsedMilliseconds -lt ($i * $cfg.PulseSpeedMs)) { [System.Threading.Thread]::Sleep(1) }
    }

    if (Test-Path $cfg.Sound2) { $p = [System.Media.SoundPlayer]::new($cfg.Sound2); $p.Play(); $players.Add($p) }
    $sw.Restart()
    for ($i=0; $i -le $cfg.WooshFrames; $i++) {
        $p = [math]::Pow(($i / $cfg.WooshFrames), $cfg.WooshEasing) 
        foreach ($form in $forms) { $form.Controls[0].Left = -[int]($form.Width * $p) }
        [System.Windows.Forms.Application]::DoEvents()
        while ($sw.ElapsedMilliseconds -lt ($i * $cfg.WooshSpeedMs)) { [System.Threading.Thread]::Sleep(1) }
    }

    # MONITOR-CHECK
    $timeout = [DateTime]::Now.AddSeconds($cfg.MaxWaitSeconds)
    while ([DateTime]::Now -lt $timeout) {
        if (Test-Path $cfg.CsvPath) {
            if ([bool](Select-String -Path $cfg.CsvPath -Pattern "TCL0000.*Yes" -Quiet) -ne $initialState) { break }
        }
        [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 150
    }

} finally {
    # RESTORE & CLEANUP
    [void][WinUtil]::LockWindowUpdate([IntPtr]::Zero)
    [void][WinUtil]::ToggleVisuals($originalState)
    
    foreach ($p in $players) { $p.Stop(); $p.Dispose() }
    foreach ($f in $forms) { if ($f) { $f.Close(); $f.Dispose() } }
    foreach ($b in $bitmaps) { if ($b) { $b.Dispose() } }
    [System.GC]::Collect()
}
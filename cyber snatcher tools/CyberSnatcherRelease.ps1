Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

# ── Config storage ────────────────────────────────────────────────────────────
$configDir = "$env:APPDATA\CyberSnatcherRelease"
$tokenFile = "$configDir\token.dat"
$configFile = "$configDir\config.json"

function EnsureConfigDir { if (-not (Test-Path $configDir)) { New-Item -ItemType Directory -Path $configDir | Out-Null } }

function SaveToken($tok) {
    EnsureConfigDir
    $tok | ConvertTo-SecureString -AsPlainText -Force | ConvertFrom-SecureString | Set-Content $tokenFile
}

function LoadToken {
    if (Test-Path $tokenFile) {
        try {
            $ss = Get-Content $tokenFile | ConvertTo-SecureString
            $bstr = [System.Runtime.InteropServices.Marshal]::SecureStringToBSTR($ss)
            return [System.Runtime.InteropServices.Marshal]::PtrToStringAuto($bstr)
        } catch { return "" }
    }
    return ""
}

function SaveConfig($data) {
    EnsureConfigDir
    $data | ConvertTo-Json | Set-Content $configFile
}

function LoadConfig {
    if (Test-Path $configFile) {
        try { return (Get-Content $configFile -Raw | ConvertFrom-Json) } catch {}
    }
    return $null
}

function GHHeaders($token) {
    return @{
        "Authorization" = "token $token"
        "Accept" = "application/vnd.github+json"
        "X-GitHub-Api-Version" = "2022-11-28"
    }
}

function TestToken($token, $repo) {
    try {
        Invoke-RestMethod -Uri "https://api.github.com/repos/$repo" -Headers (GHHeaders $token) | Out-Null
        return $true
    } catch { return $false }
}

# ── Load saved data ───────────────────────────────────────────────────────────
$savedToken = LoadToken
$savedConfig = LoadConfig
$defaultFolder = if ($savedConfig -and $savedConfig.folder) { $savedConfig.folder } else { (Get-Location).Path }
$defaultRepo = if ($savedConfig -and $savedConfig.repo) { $savedConfig.repo } else { "shaa123/CyberSnatcher" }
$defaultCloneUrl = if ($savedConfig -and $savedConfig.cloneUrl) { $savedConfig.cloneUrl } else { "https://github.com/shaa123/CyberSnatcher.git" }
$defaultBranch = if ($savedConfig -and $savedConfig.branch) { $savedConfig.branch } else { "main" }

# ── Colors (Purple Cyberpunk) ─────────────────────────────────────────────────
$bgDark    = [System.Drawing.Color]::FromArgb(10, 10, 15)
$bgPanel   = [System.Drawing.Color]::FromArgb(18, 17, 26)
$bgInput   = [System.Drawing.Color]::FromArgb(26, 23, 38)
$borderDim = [System.Drawing.Color]::FromArgb(42, 37, 64)
$borderLit = [System.Drawing.Color]::FromArgb(139, 92, 246)
$purple    = [System.Drawing.Color]::FromArgb(139, 92, 246)
$purpleDk  = [System.Drawing.Color]::FromArgb(88, 55, 160)
$purpleDim = [System.Drawing.Color]::FromArgb(50, 30, 90)
$accent    = [System.Drawing.Color]::FromArgb(192, 132, 252)
$green     = [System.Drawing.Color]::FromArgb(74, 222, 128)
$yellow    = [System.Drawing.Color]::FromArgb(251, 191, 36)
$red       = [System.Drawing.Color]::FromArgb(248, 113, 113)
$redDark   = [System.Drawing.Color]::FromArgb(140, 25, 25)
$dimText   = [System.Drawing.Color]::FromArgb(139, 134, 160)
$white     = [System.Drawing.Color]::FromArgb(232, 230, 240)
$blue      = [System.Drawing.Color]::FromArgb(96, 165, 250)
$blueDark  = [System.Drawing.Color]::FromArgb(30, 64, 120)
$blueBorder = [System.Drawing.Color]::FromArgb(96, 165, 250)

# ── Helper constructors ───────────────────────────────────────────────────────
function MkLabel($text, $x, $y, $w=540, $color=$dimText) {
    $l = New-Object System.Windows.Forms.Label
    $l.Text = $text; $l.Location = [System.Drawing.Point]::new($x,$y)
    $l.Size = [System.Drawing.Size]::new($w, 18); $l.ForeColor = $color
    return $l
}

function MkTextBox($x, $y, $w, $pass=$false) {
    $t = New-Object System.Windows.Forms.TextBox
    $t.Location = [System.Drawing.Point]::new($x,$y)
    $t.Size = [System.Drawing.Size]::new($w, 26)
    $t.BackColor = $bgInput; $t.ForeColor = $white; $t.BorderStyle = "FixedSingle"
    if ($pass) { $t.UseSystemPasswordChar = $true }
    return $t
}

function MkMultiBox($x, $y, $w, $h) {
    $t = New-Object System.Windows.Forms.TextBox
    $t.Location = [System.Drawing.Point]::new($x,$y)
    $t.Size = [System.Drawing.Size]::new($w, $h)
    $t.Multiline = $true; $t.ScrollBars = "Vertical"
    $t.BackColor = $bgInput; $t.ForeColor = $white; $t.BorderStyle = "FixedSingle"
    return $t
}

function MkButton($text, $x, $y, $w, $h, $style="normal") {
    $b = New-Object System.Windows.Forms.Button
    $b.Text = $text; $b.Location = [System.Drawing.Point]::new($x,$y)
    $b.Size = [System.Drawing.Size]::new($w, $h)
    $b.FlatStyle = "Flat"; $b.FlatAppearance.BorderSize = 1; $b.ForeColor = $white
    $b.Cursor = [System.Windows.Forms.Cursors]::Hand
    switch ($style) {
        "purple" { $b.BackColor = $purpleDk;  $b.FlatAppearance.BorderColor = $purple }
        "blue"   { $b.BackColor = $blueDark;  $b.FlatAppearance.BorderColor = $blueBorder }
        "green"  { $b.BackColor = [System.Drawing.Color]::FromArgb(20,100,60); $b.FlatAppearance.BorderColor = $green }
        "red"    { $b.BackColor = $redDark;   $b.FlatAppearance.BorderColor = $red }
        default  { $b.BackColor = $bgPanel;   $b.FlatAppearance.BorderColor = $borderDim }
    }
    return $b
}

function MkCheck($text, $x, $y, $w=300, $checked=$false) {
    $c = New-Object System.Windows.Forms.CheckBox
    $c.Text = $text; $c.Location = [System.Drawing.Point]::new($x,$y)
    $c.Size = [System.Drawing.Size]::new($w, 22)
    $c.ForeColor = $dimText; $c.BackColor = $bgDark; $c.Checked = $checked
    return $c
}

function PickFolder($startPath) {
    try {
        Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class FolderPickerV2 {
    [DllImport("shell32.dll", CharSet=CharSet.Unicode)]
    private static extern int SHCreateItemFromParsingName(string pszPath, IntPtr pbc, ref Guid riid, out IntPtr ppv);
    public static string Pick() {
        Type t = Type.GetTypeFromCLSID(new Guid("DC1C5A9C-E88A-4DDE-A5A1-60F82A20AEF7"));
        dynamic dialog = Activator.CreateInstance(t);
        dialog.SetOptions(0x20);
        dialog.SetTitle("Select folder");
        try { dialog.Show(IntPtr.Zero); } catch { return null; }
        dynamic item = dialog.GetResult();
        return item.GetDisplayName(0x80058000);
    }
}
"@ -ErrorAction Stop
        return [FolderPickerV2]::Pick()
    } catch {
        $dlg = New-Object System.Windows.Forms.FolderBrowserDialog
        if ($dlg.ShowDialog() -eq "OK") { return $dlg.SelectedPath }
        return $null
    }
}

function PickFiles {
    $dlg = New-Object System.Windows.Forms.OpenFileDialog
    $dlg.Multiselect = $true
    $dlg.Title = "Select files to upload"
    $dlg.Filter = "All Files (*.*)|*.*"
    if ($dlg.ShowDialog() -eq "OK") { return $dlg.FileNames }
    return $null
}

# ── Main Form ─────────────────────────────────────────────────────────────────
$form = New-Object System.Windows.Forms.Form
$form.Text = "CyberSnatcher Release Manager"
$form.Size = [System.Drawing.Size]::new(640, 820)
$form.StartPosition = "CenterScreen"
$form.BackColor = $bgDark; $form.ForeColor = $white
$form.FormBorderStyle = "FixedSingle"; $form.MaximizeBox = $false
$form.Font = New-Object System.Drawing.Font("Consolas", 9)

# ── Title bar ─────────────────────────────────────────────────────────────────
$lblTitle = New-Object System.Windows.Forms.Label
$lblTitle.Text = "CYBERSNATCHER RELEASE MANAGER"
$lblTitle.Location = [System.Drawing.Point]::new(10, 8)
$lblTitle.Size = [System.Drawing.Size]::new(450, 22)
$lblTitle.Font = New-Object System.Drawing.Font("Consolas", 11, [System.Drawing.FontStyle]::Bold)
$lblTitle.ForeColor = $purple
$form.Controls.Add($lblTitle)

# ── Token + Repo bar (always visible) ─────────────────────────────────────────
$pnlTop = New-Object System.Windows.Forms.Panel
$pnlTop.Location = [System.Drawing.Point]::new(10, 35)
$pnlTop.Size = [System.Drawing.Size]::new(600, 100)
$pnlTop.BackColor = $bgPanel
$form.Controls.Add($pnlTop)

$pnlTop.Controls.Add((MkLabel "GITHUB TOKEN" 10 8 100))

$lblTokStatus = New-Object System.Windows.Forms.Label
$lblTokStatus.Location = [System.Drawing.Point]::new(120, 8)
$lblTokStatus.Size = [System.Drawing.Size]::new(80, 18)
$lblTokStatus.Font = New-Object System.Drawing.Font("Consolas", 8)
$lblTokStatus.ForeColor = if ($savedToken) { $green } else { $dimText }
$lblTokStatus.Text = if ($savedToken) { "SAVED" } else { "Not set" }
$pnlTop.Controls.Add($lblTokStatus)

$txtToken = MkTextBox 10 28 400 $true
$txtToken.Text = $savedToken
$pnlTop.Controls.Add($txtToken)

$btnShowTok = MkButton "Show" 418 26 50 28
$btnShowTok.Add_Click({
    $txtToken.UseSystemPasswordChar = -not $txtToken.UseSystemPasswordChar
    $btnShowTok.Text = if ($txtToken.UseSystemPasswordChar) { "Show" } else { "Hide" }
})
$pnlTop.Controls.Add($btnShowTok)

$btnSaveTok = MkButton "Save" 475 26 55 28 "green"
$btnSaveTok.Add_Click({
    $tok = $txtToken.Text.Trim()
    if ($tok) {
        SaveToken $tok
        $lblTokStatus.Text = "SAVED"; $lblTokStatus.ForeColor = $green
        Log "Token saved."
    }
})
$pnlTop.Controls.Add($btnSaveTok)

$btnTestTok = MkButton "Test" 537 26 55 28 "blue"
$btnTestTok.Add_Click({
    $tok = $txtToken.Text.Trim()
    $repo = $txtRepo.Text.Trim()
    if (-not $tok -or -not $repo) { Log "Enter token + repo first."; return }
    if (TestToken $tok $repo) {
        $lblTokStatus.Text = "VALID"; $lblTokStatus.ForeColor = $green
        Log "Token is valid for $repo"
    } else {
        $lblTokStatus.Text = "INVALID"; $lblTokStatus.ForeColor = $red
        Log "Token failed for $repo"
    }
})
$pnlTop.Controls.Add($btnTestTok)

$pnlTop.Controls.Add((MkLabel "REPO (owner/repo)" 10 62 140))
$txtRepo = MkTextBox 10 72 290
$txtRepo.Text = $defaultRepo
$pnlTop.Controls.Add($txtRepo)

# ── Tab control ───────────────────────────────────────────────────────────────
$tabs = New-Object System.Windows.Forms.TabControl
$tabs.Location = [System.Drawing.Point]::new(10, 145)
$tabs.Size = [System.Drawing.Size]::new(600, 520)
$tabs.Font = New-Object System.Drawing.Font("Consolas", 9, [System.Drawing.FontStyle]::Bold)
$form.Controls.Add($tabs)

# ══════════════════════════════════════════════════════════════════════════════
# TAB 1: BUILD + RELEASE
# ══════════════════════════════════════════════════════════════════════════════
$tabBuild = New-Object System.Windows.Forms.TabPage
$tabBuild.Text = " Build & Release "
$tabBuild.BackColor = $bgDark
$tabs.TabPages.Add($tabBuild)

$tabBuild.Controls.Add((MkLabel "PROJECT FOLDER" 10 10))
$txtFolder = MkTextBox 10 28 470
$txtFolder.Text = $defaultFolder
$tabBuild.Controls.Add($txtFolder)

$btnBrowse = MkButton "Browse" 488 26 90 28
$btnBrowse.Add_Click({
    $picked = PickFolder $txtFolder.Text
    if ($picked) { $txtFolder.Text = $picked }
})
$tabBuild.Controls.Add($btnBrowse)

$tabBuild.Controls.Add((MkLabel "CLONE URL" 10 60))
$txtCloneUrl = MkTextBox 10 78 580
$txtCloneUrl.Text = $defaultCloneUrl
$tabBuild.Controls.Add($txtCloneUrl)

$tabBuild.Controls.Add((MkLabel "BRANCH" 10 108 80))
$txtBranch = MkTextBox 10 126 150
$txtBranch.Text = $defaultBranch
$tabBuild.Controls.Add($txtBranch)

$tabBuild.Controls.Add((MkLabel "TAG" 175 108 50))
$txtTag = MkTextBox 175 126 100
$txtTag.Text = "v1.0.0"
$tabBuild.Controls.Add($txtTag)

$tabBuild.Controls.Add((MkLabel "TITLE" 290 108 60))
$txtTitle = MkTextBox 290 126 290
$txtTitle.Text = "CyberSnatcher Release"
$tabBuild.Controls.Add($txtTitle)

$tabBuild.Controls.Add((MkLabel "RELEASE NOTES" 10 158))
$txtNotes = MkMultiBox 10 176 580 55
$txtNotes.Text = "- Bug fixes and improvements"
$tabBuild.Controls.Add($txtNotes)

$chkPre = MkCheck "Pre-release" 10 240 130
$tabBuild.Controls.Add($chkPre)
$chkPull = MkCheck "Pull latest first" 150 240 160 $true
$tabBuild.Controls.Add($chkPull)
$chkFreshClone = MkCheck "Fresh clone (deletes folder!)" 320 240 260 $true
$chkFreshClone.ForeColor = $yellow
$tabBuild.Controls.Add($chkFreshClone)

# Build buttons
$btnDevTest = MkButton "TEST (Dev Build)" 10 275 280 40 "blue"
$btnDevTest.Font = New-Object System.Drawing.Font("Consolas", 10, [System.Drawing.FontStyle]::Bold)
$tabBuild.Controls.Add($btnDevTest)

$btnRelease = MkButton "BUILD + RELEASE" 300 275 280 40 "purple"
$btnRelease.Font = New-Object System.Drawing.Font("Consolas", 10, [System.Drawing.FontStyle]::Bold)
$tabBuild.Controls.Add($btnRelease)

# Build log
$txtBuildLog = New-Object System.Windows.Forms.TextBox
$txtBuildLog.Location = [System.Drawing.Point]::new(10, 325)
$txtBuildLog.Size = [System.Drawing.Size]::new(580, 155)
$txtBuildLog.Multiline = $true; $txtBuildLog.ScrollBars = "Vertical"; $txtBuildLog.ReadOnly = $true
$txtBuildLog.BackColor = [System.Drawing.Color]::FromArgb(10,10,15)
$txtBuildLog.ForeColor = $accent; $txtBuildLog.BorderStyle = "FixedSingle"
$txtBuildLog.Text = "Ready."
$tabBuild.Controls.Add($txtBuildLog)

# ══════════════════════════════════════════════════════════════════════════════
# TAB 2: PUSH FILES
# ══════════════════════════════════════════════════════════════════════════════
$tabPush = New-Object System.Windows.Forms.TabPage
$tabPush.Text = " Push Files "
$tabPush.BackColor = $bgDark
$tabs.TabPages.Add($tabPush)

$tabPush.Controls.Add((MkLabel "Upload any files to a GitHub repo (creates/overwrites)" 10 10 580))

$tabPush.Controls.Add((MkLabel "REPO PATH (where in the repo, e.g. assets/ or leave blank for root)" 10 38 580))
$txtPushPath = MkTextBox 10 56 580
$txtPushPath.Text = ""
$tabPush.Controls.Add($txtPushPath)

$tabPush.Controls.Add((MkLabel "COMMIT MESSAGE" 10 88))
$txtCommitMsg = MkTextBox 10 106 580
$txtCommitMsg.Text = "Upload files"
$tabPush.Controls.Add($txtCommitMsg)

$tabPush.Controls.Add((MkLabel "BRANCH" 10 138))
$txtPushBranch = MkTextBox 10 156 200
$txtPushBranch.Text = "main"
$tabPush.Controls.Add($txtPushBranch)

# File list
$tabPush.Controls.Add((MkLabel "FILES TO PUSH" 10 190))
$lstPushFiles = New-Object System.Windows.Forms.ListBox
$lstPushFiles.Location = [System.Drawing.Point]::new(10, 208)
$lstPushFiles.Size = [System.Drawing.Size]::new(580, 120)
$lstPushFiles.BackColor = $bgInput; $lstPushFiles.ForeColor = $white
$lstPushFiles.BorderStyle = "FixedSingle"; $lstPushFiles.SelectionMode = "MultiExtended"
$tabPush.Controls.Add($lstPushFiles)

$btnAddFiles = MkButton "+ Add Files" 10 335 140 30 "blue"
$btnAddFiles.Add_Click({
    $files = PickFiles
    if ($files) {
        foreach ($f in $files) {
            if (-not $lstPushFiles.Items.Contains($f)) {
                $lstPushFiles.Items.Add($f)
            }
        }
    }
})
$tabPush.Controls.Add($btnAddFiles)

$btnRemoveFiles = MkButton "- Remove Selected" 160 335 160 30
$btnRemoveFiles.Add_Click({
    $selected = @($lstPushFiles.SelectedItems)
    foreach ($s in $selected) { $lstPushFiles.Items.Remove($s) }
})
$tabPush.Controls.Add($btnRemoveFiles)

$btnClearFiles = MkButton "Clear All" 330 335 100 30
$btnClearFiles.Add_Click({ $lstPushFiles.Items.Clear() })
$tabPush.Controls.Add($btnClearFiles)

# Push log
$txtPushLog = New-Object System.Windows.Forms.TextBox
$txtPushLog.Location = [System.Drawing.Point]::new(10, 405)
$txtPushLog.Size = [System.Drawing.Size]::new(580, 75)
$txtPushLog.Multiline = $true; $txtPushLog.ScrollBars = "Vertical"; $txtPushLog.ReadOnly = $true
$txtPushLog.BackColor = [System.Drawing.Color]::FromArgb(10,10,15)
$txtPushLog.ForeColor = $accent; $txtPushLog.BorderStyle = "FixedSingle"
$txtPushLog.Text = "Ready."
$tabPush.Controls.Add($txtPushLog)

$btnPush = MkButton "PUSH FILES TO REPO" 10 375 580 28 "green"
$btnPush.Font = New-Object System.Drawing.Font("Consolas", 10, [System.Drawing.FontStyle]::Bold)
$tabPush.Controls.Add($btnPush)

# ══════════════════════════════════════════════════════════════════════════════
# TAB 3: DELETE FILES FROM REPO
# ══════════════════════════════════════════════════════════════════════════════
$tabDelete = New-Object System.Windows.Forms.TabPage
$tabDelete.Text = " Delete from Repo "
$tabDelete.BackColor = $bgDark
$tabs.TabPages.Add($tabDelete)

$tabDelete.Controls.Add((MkLabel "Browse and delete files from your GitHub repo" 10 10 580))

$tabDelete.Controls.Add((MkLabel "BRANCH" 10 35))
$txtDelBranch = MkTextBox 10 53 200
$txtDelBranch.Text = "main"
$tabDelete.Controls.Add($txtDelBranch)

$tabDelete.Controls.Add((MkLabel "PATH FILTER (e.g. assets/ or blank for root)" 220 35 370))
$txtDelPath = MkTextBox 220 53 200
$tabDelete.Controls.Add($txtDelPath)

$btnListFiles = MkButton "Fetch File List" 430 51 150 28 "blue"
$tabDelete.Controls.Add($btnListFiles)

$tabDelete.Controls.Add((MkLabel "FILES IN REPO (select to delete)" 10 85))
$lstRepoFiles = New-Object System.Windows.Forms.ListBox
$lstRepoFiles.Location = [System.Drawing.Point]::new(10, 103)
$lstRepoFiles.Size = [System.Drawing.Size]::new(580, 220)
$lstRepoFiles.BackColor = $bgInput; $lstRepoFiles.ForeColor = $white
$lstRepoFiles.BorderStyle = "FixedSingle"; $lstRepoFiles.SelectionMode = "MultiExtended"
$tabDelete.Controls.Add($lstRepoFiles)

$txtDelLog = New-Object System.Windows.Forms.TextBox
$txtDelLog.Location = [System.Drawing.Point]::new(10, 375)
$txtDelLog.Size = [System.Drawing.Size]::new(580, 105)
$txtDelLog.Multiline = $true; $txtDelLog.ScrollBars = "Vertical"; $txtDelLog.ReadOnly = $true
$txtDelLog.BackColor = [System.Drawing.Color]::FromArgb(10,10,15)
$txtDelLog.ForeColor = $accent; $txtDelLog.BorderStyle = "FixedSingle"
$txtDelLog.Text = "Ready."
$tabDelete.Controls.Add($txtDelLog)

$btnDeleteFiles = MkButton "DELETE SELECTED FILES" 10 335 580 35 "red"
$btnDeleteFiles.Font = New-Object System.Drawing.Font("Consolas", 10, [System.Drawing.FontStyle]::Bold)
$tabDelete.Controls.Add($btnDeleteFiles)

# ── Status bar ────────────────────────────────────────────────────────────────
$lblStatus = New-Object System.Windows.Forms.Label
$lblStatus.Location = [System.Drawing.Point]::new(10, 670)
$lblStatus.Size = [System.Drawing.Size]::new(600, 20)
$lblStatus.ForeColor = $dimText
$lblStatus.Text = ""
$form.Controls.Add($lblStatus)

# ── Log helper ────────────────────────────────────────────────────────────────
function Log($msg, $target=$null) {
    if (-not $target) {
        $activeTab = $tabs.SelectedIndex
        switch ($activeTab) {
            0 { $target = $txtBuildLog }
            1 { $target = $txtPushLog }
            2 { $target = $txtDelLog }
            3 { $target = $txtSyncLog }
        }
    }
    if ($target) {
        $target.AppendText("`r`n$msg")
        $target.ScrollToCaret()
    }
    $lblStatus.Text = $msg
    $form.Refresh()
}

function SaveAll {
    SaveConfig @{
        folder = $txtFolder.Text.Trim()
        repo = $txtRepo.Text.Trim()
        cloneUrl = $txtCloneUrl.Text.Trim()
        branch = $txtBranch.Text.Trim()
    }
}

# ══════════════════════════════════════════════════════════════════════════════
# BUILD + RELEASE LOGIC
# ══════════════════════════════════════════════════════════════════════════════

function DoCloneOrPull($folder, $token) {
    if ($chkFreshClone.Checked) {
        $cloneUrl = $txtCloneUrl.Text.Trim()
        if (-not $cloneUrl) { throw "Clone URL is empty" }
        $cloneUrl = $cloneUrl -replace "/tree/.*$", ""
        if (-not $cloneUrl.EndsWith(".git")) { $cloneUrl = $cloneUrl + ".git" }
        if ($token -and $cloneUrl -match "^https://github.com/") {
            $cloneUrl = $cloneUrl -replace "^https://", "https://$token@"
        }
        $branch = $txtBranch.Text.Trim()
        $parent = Split-Path $folder -Parent
        $dirName = Split-Path $folder -Leaf

        # Move out of the folder first so we can delete it
        Set-Location $parent

        if (Test-Path $folder) {
            Log "Deleting $folder..."
            try {
                Remove-Item -Recurse -Force $folder -ErrorAction Stop
            } catch {
                # Folder locked — wipe contents instead
                Log "Folder in use, clearing contents instead..."
                Get-ChildItem -Path $folder -Force | Where-Object {
                    $_.Name -notin @('CyberSnatcherRelease.ps1','CyberSnatcherRelease.bat','CyberVaultRelease.ps1','CyberVaultRelease.bat')
                } | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
                # Remove .git so clone can reinitialize
                $gitDir = Join-Path $folder ".git"
                if (Test-Path $gitDir) {
                    attrib -h -r -s "$gitDir\*.*" /s /d 2>$null
                    Remove-Item -Recurse -Force $gitDir -ErrorAction SilentlyContinue
                }
            }
        }

        Log "Cloning branch $branch..."
        if (Test-Path $folder) {
            # Folder still exists (couldn't fully delete) — clone into temp then move
            $tempName = "$dirName-clone-temp"
            $tempPath = Join-Path $parent $tempName
            if (Test-Path $tempPath) { Remove-Item -Recurse -Force $tempPath }
            Start-Process "cmd.exe" "/c git clone --branch `"$branch`" `"$cloneUrl`" `"$tempName`"" -WorkingDirectory $parent -Wait -NoNewWindow
            if (Test-Path $tempPath) {
                # Copy cloned files into the original folder
                Log "Moving cloned files into project folder..."
                Get-ChildItem -Path $tempPath -Force | Copy-Item -Destination $folder -Recurse -Force
                Remove-Item -Recurse -Force $tempPath -ErrorAction SilentlyContinue
            }
        } else {
            Start-Process "cmd.exe" "/c git clone --branch `"$branch`" `"$cloneUrl`" `"$dirName`"" -WorkingDirectory $parent -Wait -NoNewWindow
        }
        Log "Cloned!"
    } elseif ($chkPull.Checked) {
        $branch = $txtBranch.Text.Trim()
        Log "Pulling $branch..."
        Start-Process "cmd.exe" "/c git fetch origin && git checkout $branch && git pull origin $branch" -WorkingDirectory $folder -Wait -NoNewWindow
        Log "Pulled!"
    }
}

$btnDevTest.Add_Click({
    $folder = $txtFolder.Text.Trim()
    if (-not $folder) { Log "Set project folder first."; return }
    $btnDevTest.Enabled = $false; $btnRelease.Enabled = $false
    $txtBuildLog.Text = "Starting dev build..."
    try {
        DoCloneOrPull $folder $txtToken.Text.Trim()
        Log "Running npm install..."
        $p = Start-Process "cmd.exe" "/c npm install" -WorkingDirectory $folder -Wait -PassThru -NoNewWindow
        if ($p.ExitCode -ne 0) { throw "npm install failed" }
        Log "Launching dev app..."
        Start-Process "cmd.exe" "/c npm run tauri dev" -WorkingDirectory $folder -NoNewWindow
        Log "Dev app launched!"
        SaveAll
    } catch { Log "ERROR: $_" }
    $btnDevTest.Enabled = $true; $btnRelease.Enabled = $true
})

$btnRelease.Add_Click({
    $folder = $txtFolder.Text.Trim()
    $token  = $txtToken.Text.Trim()
    $repo   = $txtRepo.Text.Trim()
    $tag    = $txtTag.Text.Trim()
    $title  = $txtTitle.Text.Trim()
    $notes  = $txtNotes.Text.Trim()

    if (-not $folder -or -not $token -or -not $repo -or -not $tag -or -not $title) {
        Log "Fill in all fields first."; return
    }

    $txtBuildLog.Text = "Validating token..."
    if (-not (TestToken $token $repo)) {
        Log "Token invalid or expired!"; return
    }
    Log "Token valid."

    $btnDevTest.Enabled = $false; $btnRelease.Enabled = $false
    try {
        DoCloneOrPull $folder $token

        Log "Running npm install..."
        $p = Start-Process "cmd.exe" "/c npm install" -WorkingDirectory $folder -Wait -PassThru -NoNewWindow
        if ($p.ExitCode -ne 0) { throw "npm install failed" }

        Log "Building release... (this takes a few minutes)"
        $p = Start-Process "cmd.exe" "/c npm run tauri build" -WorkingDirectory $folder -Wait -PassThru -NoNewWindow
        if ($p.ExitCode -ne 0) { throw "Build failed" }
        Log "Build complete!"

        Log "Finding installer..."
        $bundleDir = Join-Path $folder "src-tauri\target\release\bundle"
        $exeFile = Get-ChildItem -Path $bundleDir -Recurse -Include "*.exe","*.msi" |
                   Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if (-not $exeFile) { throw "No installer found in $bundleDir" }
        Log "Found: $($exeFile.Name)"

        Log "Creating GitHub release $tag..."
        $headers = GHHeaders $token
        $releaseBody = @{
            tag_name = $tag; name = $title; body = $notes
            draft = $false; prerelease = $chkPre.Checked
        } | ConvertTo-Json
        $resp = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases" `
            -Method Post -Headers $headers -Body $releaseBody -ContentType "application/json"
        $uploadUrl = $resp.upload_url -replace "\{\?name,label\}", ""
        Log "Release created!"

        Log "Uploading $($exeFile.Name)..."
        $upHeaders = @{ "Authorization" = "token $token"; "Content-Type" = "application/octet-stream" }
        Invoke-RestMethod -Uri "$($uploadUrl)?name=$([System.Uri]::EscapeDataString($exeFile.Name))" `
            -Method Post -Headers $upHeaders -InFile $exeFile.FullName | Out-Null
        Log "Uploaded!"
        Log ""
        Log "DONE! https://github.com/$repo/releases/tag/$tag"

        SaveToken $token; SaveAll
        $lblTokStatus.Text = "SAVED"; $lblTokStatus.ForeColor = $green
    } catch { Log "ERROR: $_" }
    $btnDevTest.Enabled = $true; $btnRelease.Enabled = $true
})

# ══════════════════════════════════════════════════════════════════════════════
# PUSH FILES LOGIC
# ══════════════════════════════════════════════════════════════════════════════
$btnPush.Add_Click({
    $token = $txtToken.Text.Trim()
    $repo  = $txtRepo.Text.Trim()
    $repoPath = $txtPushPath.Text.Trim().TrimEnd("/").TrimEnd("\")
    $commitMsg = $txtCommitMsg.Text.Trim()
    $branch = $txtPushBranch.Text.Trim()

    if (-not $token -or -not $repo) { Log "Set token + repo first." $txtPushLog; return }
    if ($lstPushFiles.Items.Count -eq 0) { Log "Add files first." $txtPushLog; return }
    if (-not $commitMsg) { $commitMsg = "Upload files" }
    if (-not $branch) { $branch = "main" }

    $txtPushLog.Text = "Pushing files..."
    $btnPush.Enabled = $false
    $headers = GHHeaders $token
    $count = 0

    try {
        foreach ($filePath in $lstPushFiles.Items) {
            $fileName = [System.IO.Path]::GetFileName($filePath)
            $ghPath = if ($repoPath) { "$repoPath/$fileName" } else { $fileName }

            Log "Uploading $fileName -> $ghPath ..." $txtPushLog

            $bytes = [System.IO.File]::ReadAllBytes($filePath)
            $b64 = [Convert]::ToBase64String($bytes)

            # Check if file already exists (need its SHA to update)
            $sha = $null
            try {
                $existing = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/contents/$ghPath`?ref=$branch" -Headers $headers
                $sha = $existing.sha
                Log "  File exists, updating..." $txtPushLog
            } catch {
                Log "  New file, creating..." $txtPushLog
            }

            $body = @{
                message = "$commitMsg - $fileName"
                content = $b64
                branch = $branch
            }
            if ($sha) { $body.sha = $sha }

            Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/contents/$ghPath" `
                -Method Put -Headers $headers -Body ($body | ConvertTo-Json) -ContentType "application/json" | Out-Null

            Log "  Done!" $txtPushLog
            $count++
        }
        Log "" $txtPushLog
        Log "$count file(s) pushed to $repo!" $txtPushLog
    } catch {
        Log "ERROR: $_" $txtPushLog
    }
    $btnPush.Enabled = $true
})

# ══════════════════════════════════════════════════════════════════════════════
# DELETE FILES LOGIC
# ══════════════════════════════════════════════════════════════════════════════
$btnListFiles.Add_Click({
    $token = $txtToken.Text.Trim()
    $repo  = $txtRepo.Text.Trim()
    $branch = $txtDelBranch.Text.Trim()
    $pathFilter = $txtDelPath.Text.Trim().TrimEnd("/")

    if (-not $token -or -not $repo) { Log "Set token + repo first." $txtDelLog; return }
    if (-not $branch) { $branch = "main" }

    $lstRepoFiles.Items.Clear()
    $txtDelLog.Text = "Fetching file list..."
    $btnListFiles.Enabled = $false

    try {
        $url = "https://api.github.com/repos/$repo/git/trees/$branch`?recursive=1"
        $tree = Invoke-RestMethod -Uri $url -Headers (GHHeaders $token)

        foreach ($item in $tree.tree) {
            if ($item.type -eq "blob") {
                if ($pathFilter -and -not $item.path.StartsWith($pathFilter)) { continue }
                $lstRepoFiles.Items.Add($item.path)
            }
        }
        Log "$($lstRepoFiles.Items.Count) files found." $txtDelLog
    } catch {
        Log "ERROR: $_" $txtDelLog
    }
    $btnListFiles.Enabled = $true
})

$btnDeleteFiles.Add_Click({
    $token = $txtToken.Text.Trim()
    $repo  = $txtRepo.Text.Trim()
    $branch = $txtDelBranch.Text.Trim()
    if (-not $branch) { $branch = "main" }

    $selected = @($lstRepoFiles.SelectedItems)
    if ($selected.Count -eq 0) { Log "Select files to delete first." $txtDelLog; return }

    $confirm = [System.Windows.Forms.MessageBox]::Show(
        "Delete $($selected.Count) file(s) from $repo?`n`nThis cannot be undone!",
        "Confirm Delete", "YesNo", "Warning")
    if ($confirm -ne "Yes") { return }

    $txtDelLog.Text = "Deleting files..."
    $btnDeleteFiles.Enabled = $false
    $headers = GHHeaders $token
    $count = 0

    try {
        foreach ($filePath in $selected) {
            Log "Deleting $filePath ..." $txtDelLog
            try {
                $fileInfo = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/contents/$filePath`?ref=$branch" -Headers $headers
                $body = @{
                    message = "Delete $filePath"
                    sha = $fileInfo.sha
                    branch = $branch
                } | ConvertTo-Json

                Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/contents/$filePath" `
                    -Method Delete -Headers $headers -Body $body -ContentType "application/json" | Out-Null
                Log "  Deleted!" $txtDelLog
                $count++
            } catch {
                Log "  FAILED: $_" $txtDelLog
            }
        }
        Log "" $txtDelLog
        Log "$count file(s) deleted from $repo." $txtDelLog

        # Refresh list
        $btnListFiles.PerformClick()
    } catch {
        Log "ERROR: $_" $txtDelLog
    }
    $btnDeleteFiles.Enabled = $true
})

# ══════════════════════════════════════════════════════════════════════════════
# TAB 4: AUTO-SYNC (FILE WATCHER)
# ══════════════════════════════════════════════════════════════════════════════
$tabSync = New-Object System.Windows.Forms.TabPage
$tabSync.Text = " Auto-Sync "
$tabSync.BackColor = $bgDark
$tabs.TabPages.Add($tabSync)

$tabSync.Controls.Add((MkLabel "Watch a local folder and auto-push changes to GitHub in real time" 10 10 580 $accent))

# Watch folder
$tabSync.Controls.Add((MkLabel "WATCH FOLDER" 10 35 120))
$txtSyncFolder = MkTextBox 10 53 470
$txtSyncFolder.Text = "C:\Users\ItsMe\Downloads\cyber Snatcher"
$tabSync.Controls.Add($txtSyncFolder)

$btnSyncBrowse = MkButton "Browse" 488 51 90 28
$btnSyncBrowse.Add_Click({
    $picked = PickFolder $txtSyncFolder.Text
    if ($picked) { $txtSyncFolder.Text = $picked }
})
$tabSync.Controls.Add($btnSyncBrowse)

# Branch for sync
$tabSync.Controls.Add((MkLabel "BRANCH" 10 85 80))
$txtSyncBranch = MkTextBox 10 103 200
$txtSyncBranch.Text = "main"
$tabSync.Controls.Add($txtSyncBranch)

# Ignore patterns
$tabSync.Controls.Add((MkLabel "IGNORE (comma-separated folders/files to skip)" 220 85 370))
$txtSyncIgnore = MkTextBox 220 103 360
$txtSyncIgnore.Text = "node_modules,.git,target,dist,.DS_Store,Thumbs.db"
$tabSync.Controls.Add($txtSyncIgnore)

# Status indicator
$lblSyncLive = New-Object System.Windows.Forms.Label
$lblSyncLive.Text = "OFF"
$lblSyncLive.Location = [System.Drawing.Point]::new(500, 138)
$lblSyncLive.Size = [System.Drawing.Size]::new(80, 22)
$lblSyncLive.Font = New-Object System.Drawing.Font("Consolas", 10, [System.Drawing.FontStyle]::Bold)
$lblSyncLive.ForeColor = $dimText
$lblSyncLive.TextAlign = "MiddleRight"
$tabSync.Controls.Add($lblSyncLive)

# Toggle button
$btnToggleSync = MkButton "START WATCHING" 10 135 240 38 "purple"
$btnToggleSync.Font = New-Object System.Drawing.Font("Consolas", 10, [System.Drawing.FontStyle]::Bold)
$tabSync.Controls.Add($btnToggleSync)

# Full sync button
$btnFullSync = MkButton "FULL SYNC NOW" 260 135 230 38 "blue"
$btnFullSync.Font = New-Object System.Drawing.Font("Consolas", 10, [System.Drawing.FontStyle]::Bold)
$tabSync.Controls.Add($btnFullSync)

# Sync log
$txtSyncLog = New-Object System.Windows.Forms.TextBox
$txtSyncLog.Location = [System.Drawing.Point]::new(10, 185)
$txtSyncLog.Size = [System.Drawing.Size]::new(580, 290)
$txtSyncLog.Multiline = $true; $txtSyncLog.ScrollBars = "Vertical"; $txtSyncLog.ReadOnly = $true
$txtSyncLog.BackColor = [System.Drawing.Color]::FromArgb(10,10,15)
$txtSyncLog.ForeColor = $accent; $txtSyncLog.BorderStyle = "FixedSingle"
$txtSyncLog.Font = New-Object System.Drawing.Font("Consolas", 8)
$txtSyncLog.Text = "Ready. Configure watch folder and click START WATCHING."
$tabSync.Controls.Add($txtSyncLog)

# ── Watcher state ─────────────────────────────────────────────────────────────
$script:watcher = $null
$script:isWatching = $false
$script:pendingChanges = @{}

function SyncLog($msg) {
    $txtSyncLog.AppendText("`r`n$msg")
    $txtSyncLog.ScrollToCaret()
    $lblStatus.Text = $msg
    $form.Refresh()
}

function ShouldIgnoreSync($filePath, $watchFolder) {
    $ignoreList = $txtSyncIgnore.Text.Trim().Split(",") | ForEach-Object { $_.Trim() } | Where-Object { $_ }
    $relativePath = $filePath
    if ($filePath.StartsWith($watchFolder)) {
        $relativePath = $filePath.Substring($watchFolder.Length).TrimStart("\", "/")
    }
    foreach ($pattern in $ignoreList) {
        # Check if any path segment matches the ignore pattern
        $segments = $relativePath -split "[/\\]"
        foreach ($seg in $segments) {
            if ($seg -eq $pattern) { return $true }
        }
        # Also check wildcard-style matches
        if ($relativePath -like "$pattern*" -or $relativePath -like "*\$pattern\*" -or $relativePath -like "*/$pattern/*") {
            return $true
        }
    }
    return $false
}

function GetRelGHPath($filePath, $watchFolder) {
    $rel = $filePath.Substring($watchFolder.TrimEnd("\", "/").Length).TrimStart("\", "/")
    return $rel -replace "\\", "/"
}

function PushFileToGH($localPath, $ghPath, $token, $repo, $branch) {
    try {
        $headers = GHHeaders $token
        $bytes = [System.IO.File]::ReadAllBytes($localPath)
        $b64 = [Convert]::ToBase64String($bytes)

        $sha = $null
        try {
            $existing = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/contents/$ghPath`?ref=$branch" -Headers $headers
            $sha = $existing.sha
        } catch {}

        $body = @{ message = "sync: update $ghPath"; content = $b64; branch = $branch }
        if ($sha) { $body.sha = $sha }

        Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/contents/$ghPath" `
            -Method Put -Headers $headers -Body ($body | ConvertTo-Json -Depth 5) -ContentType "application/json" | Out-Null

        $action = if ($sha) { "updated" } else { "created" }
        SyncLog "[PUSH] $action $ghPath"
    } catch {
        SyncLog "[ERR] Push failed: $ghPath -- $_"
    }
}

function DeleteFileFromGH($ghPath, $token, $repo, $branch) {
    try {
        $headers = GHHeaders $token
        $fileInfo = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/contents/$ghPath`?ref=$branch" -Headers $headers
        $body = @{ message = "sync: delete $ghPath"; sha = $fileInfo.sha; branch = $branch } | ConvertTo-Json

        Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/contents/$ghPath" `
            -Method Delete -Headers $headers -Body $body -ContentType "application/json" | Out-Null

        SyncLog "[DEL] Removed $ghPath"
    } catch {
        SyncLog "[DEL] $ghPath not on remote (already gone?)"
    }
}

# ── Debounce timer — waits 2s after last change before syncing ────────────────
$script:debounceTimer = New-Object System.Windows.Forms.Timer
$script:debounceTimer.Interval = 2000
$script:debounceTimer.Add_Tick({
    $script:debounceTimer.Stop()

    $token = $txtToken.Text.Trim()
    $repo = $txtRepo.Text.Trim()
    $branch = $txtSyncBranch.Text.Trim()
    $watchFolder = $txtSyncFolder.Text.Trim().TrimEnd("\", "/")

    if (-not $token -or -not $repo) { return }

    $changes = @{}
    foreach ($k in @($script:pendingChanges.Keys)) { $changes[$k] = $script:pendingChanges[$k] }
    $script:pendingChanges = @{}

    foreach ($ghPath in $changes.Keys) {
        $action = $changes[$ghPath]

        if ($action -eq "delete") {
            DeleteFileFromGH $ghPath $token $repo $branch
        } else {
            $localPath = Join-Path $watchFolder ($ghPath -replace "/", "\")
            if (Test-Path $localPath) {
                PushFileToGH $localPath $ghPath $token $repo $branch
            }
        }
    }
})

# ── Full sync: push all local, delete remote-only files ───────────────────────
$btnFullSync.Add_Click({
    $watchFolder = $txtSyncFolder.Text.Trim().TrimEnd("\", "/")
    $token = $txtToken.Text.Trim()
    $repo = $txtRepo.Text.Trim()
    $branch = $txtSyncBranch.Text.Trim()

    if (-not $token -or -not $repo) { SyncLog "[ERR] Set token + repo at the top first."; return }
    if (-not $watchFolder -or -not (Test-Path $watchFolder)) { SyncLog "[ERR] Watch folder doesn't exist."; return }
    if (-not $branch) { $branch = "main" }

    $btnFullSync.Enabled = $false
    SyncLog "[SYNC] Starting full sync of $watchFolder..."

    # Get remote tree
    $remoteFiles = @{}
    try {
        $tree = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/git/trees/$branch`?recursive=1" -Headers (GHHeaders $token)
        foreach ($item in $tree.tree) {
            if ($item.type -eq "blob") { $remoteFiles[$item.path] = $item.sha }
        }
        SyncLog "[SYNC] $($remoteFiles.Count) files on remote."
    } catch {
        SyncLog "[ERR] Could not fetch remote: $_"
        $btnFullSync.Enabled = $true
        return
    }

    # Push all local files
    $localFiles = Get-ChildItem -Path $watchFolder -Recurse -File -ErrorAction SilentlyContinue
    $localPaths = @{}
    $pushed = 0; $deleted = 0

    foreach ($f in $localFiles) {
        if (ShouldIgnoreSync $f.FullName $watchFolder) { continue }
        $ghPath = GetRelGHPath $f.FullName $watchFolder
        $localPaths[$ghPath] = $true
        PushFileToGH $f.FullName $ghPath $token $repo $branch
        $pushed++
    }

    # Delete files that exist on remote but not locally
    foreach ($remotePath in $remoteFiles.Keys) {
        if (ShouldIgnoreSync $remotePath "") { continue }
        if (-not $localPaths.ContainsKey($remotePath)) {
            DeleteFileFromGH $remotePath $token $repo $branch
            $deleted++
        }
    }

    SyncLog "[SYNC] Done! $pushed pushed, $deleted deleted."
    $btnFullSync.Enabled = $true
})

# ── Start / Stop watcher toggle ──────────────────────────────────────────────
$btnToggleSync.Add_Click({
    if ($script:isWatching) {
        # STOP
        if ($script:watcher) {
            $script:watcher.EnableRaisingEvents = $false
            $script:watcher.Dispose()
            $script:watcher = $null
        }
        $script:debounceTimer.Stop()
        $script:pendingChanges = @{}
        $script:isWatching = $false

        $btnToggleSync.Text = "START WATCHING"
        $btnToggleSync.BackColor = $purpleDk
        $btnToggleSync.FlatAppearance.BorderColor = $purple
        $btnToggleSync.ForeColor = $white
        $lblSyncLive.Text = "OFF"
        $lblSyncLive.ForeColor = $dimText

        SyncLog "[SYNC] Watcher stopped."
    } else {
        # START
        $watchFolder = $txtSyncFolder.Text.Trim().TrimEnd("\", "/")
        $token = $txtToken.Text.Trim()
        $repo = $txtRepo.Text.Trim()

        if (-not $token -or -not $repo) { SyncLog "[ERR] Set token + repo at the top first."; return }
        if (-not $watchFolder -or -not (Test-Path $watchFolder)) { SyncLog "[ERR] Watch folder doesn't exist."; return }

        $script:watcher = New-Object System.IO.FileSystemWatcher
        $script:watcher.Path = $watchFolder
        $script:watcher.IncludeSubdirectories = $true
        $script:watcher.EnableRaisingEvents = $false
        $script:watcher.NotifyFilter = [System.IO.NotifyFilters]::FileName -bor
                                       [System.IO.NotifyFilters]::DirectoryName -bor
                                       [System.IO.NotifyFilters]::LastWrite -bor
                                       [System.IO.NotifyFilters]::Size
        # Marshal events to the UI thread so we can safely touch form controls
        $script:watcher.SynchronizingObject = $form

        $script:watcher.Add_Changed({
            $p = $_.FullPath; $wf = $txtSyncFolder.Text.Trim().TrimEnd("\", "/")
            if (ShouldIgnoreSync $p $wf) { return }
            $ghP = GetRelGHPath $p $wf
            if ($ghP) { $script:pendingChanges[$ghP] = "push"; SyncLog "[WATCH] Queued push: $ghP"; $script:debounceTimer.Stop(); $script:debounceTimer.Start() }
        })
        $script:watcher.Add_Created({
            $p = $_.FullPath; $wf = $txtSyncFolder.Text.Trim().TrimEnd("\", "/")
            if (ShouldIgnoreSync $p $wf) { return }
            $ghP = GetRelGHPath $p $wf
            if ($ghP) { $script:pendingChanges[$ghP] = "push"; SyncLog "[WATCH] Queued push: $ghP"; $script:debounceTimer.Stop(); $script:debounceTimer.Start() }
        })
        $script:watcher.Add_Deleted({
            $p = $_.FullPath; $wf = $txtSyncFolder.Text.Trim().TrimEnd("\", "/")
            if (ShouldIgnoreSync $p $wf) { return }
            $ghP = GetRelGHPath $p $wf
            if ($ghP) { $script:pendingChanges[$ghP] = "delete"; SyncLog "[WATCH] Queued delete: $ghP"; $script:debounceTimer.Stop(); $script:debounceTimer.Start() }
        })
        $script:watcher.Add_Renamed({
            $p = $_.FullPath; $old = $_.OldFullPath; $wf = $txtSyncFolder.Text.Trim().TrimEnd("\", "/")
            if (-not (ShouldIgnoreSync $old $wf)) {
                $ghOld = GetRelGHPath $old $wf
                if ($ghOld) { $script:pendingChanges[$ghOld] = "delete" ; SyncLog "[WATCH] Queued delete (rename): $ghOld" }
            }
            if (-not (ShouldIgnoreSync $p $wf)) {
                $ghP = GetRelGHPath $p $wf
                if ($ghP) { $script:pendingChanges[$ghP] = "push"; SyncLog "[WATCH] Queued push (rename): $ghP" }
            }
            $script:debounceTimer.Stop(); $script:debounceTimer.Start()
        })

        $script:watcher.EnableRaisingEvents = $true
        $script:isWatching = $true

        $btnToggleSync.Text = "STOP WATCHING"
        $btnToggleSync.BackColor = $redDark
        $btnToggleSync.FlatAppearance.BorderColor = $red
        $btnToggleSync.ForeColor = $red
        $lblSyncLive.Text = "LIVE"
        $lblSyncLive.ForeColor = $green

        SyncLog "[SYNC] Watcher started on $watchFolder"
        SyncLog "[SYNC] Using token + repo from top panel -> $repo"
        SyncLog "[SYNC] Changes auto-push to branch: $($txtSyncBranch.Text.Trim())"
    }
})

# Clean up watcher on form close
$form.Add_FormClosing({
    if ($script:isWatching -and $script:watcher) {
        $script:watcher.EnableRaisingEvents = $false
        $script:watcher.Dispose()
    }
})

# ── Show form ─────────────────────────────────────────────────────────────────
$form.ShowDialog()

\xEF\xBB\xBF\xEF\xBB\xBF# ═══════════════════════════════════════════════════════════════════════
#  CyberSnatcher Dev Launcher — PowerShell + WPF
#  Cute pastel warm night mode with real animations
#
#  Features:
#    - Splash screen with fade-out animation
#    - Animated spinner (rotating + pulsing)
#    - Button hover glow animations
#    - Fade-in transitions on all elements
#    - Background glow orbs
#    - Real-time log output with auto-scroll
#    - Status indicators (idle/running/success/error)
#    - Sound effect on completion
#    - Non-blocking command execution via runspace
# ═══════════════════════════════════════════════════════════════════════

Add-Type -AssemblyName PresentationFramework
Add-Type -AssemblyName PresentationCore
Add-Type -AssemblyName WindowsBase
Add-Type -AssemblyName System.Windows.Forms

# ─── CONFIG ─────────────────────────────────────────────
$WorkDir = "C:\Users\ItsMe\Downloads\CyberSnatcher"

# ─── XAML UI DEFINITION ─────────────────────────────────
[xml]$xaml = @"
<Window
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    Title="CyberSnatcher Launcher"
    Width="780" Height="620"
    MinWidth="600" MinHeight="480"
    WindowStartupLocation="CenterScreen"
    Background="#FF13101C"
    FontFamily="Segoe UI"
    FontSize="14"
    Foreground="#FFF0E8F8"
    ResizeMode="CanResizeWithGrip"
    AllowsTransparency="False">

    <!-- ═══ WINDOW RESOURCES (styles, animations, brushes) ═══ -->
    <Window.Resources>

        <!-- Pastel color brushes -->
        <SolidColorBrush x:Key="BgDeep" Color="#FF13101C"/>
        <SolidColorBrush x:Key="BgCard" Color="#FF211A30"/>
        <SolidColorBrush x:Key="BgInput" Color="#FF18122A"/>
        <SolidColorBrush x:Key="Pink" Color="#FFF0A0C0"/>
        <SolidColorBrush x:Key="Purple" Color="#FFC8A0E8"/>
        <SolidColorBrush x:Key="Blue" Color="#FFA0B8E8"/>
        <SolidColorBrush x:Key="Warm" Color="#FFE8C0A0"/>
        <SolidColorBrush x:Key="Mint" Color="#FF88D8A8"/>
        <SolidColorBrush x:Key="TextMuted" Color="#FFA898B8"/>
        <SolidColorBrush x:Key="TextDim" Color="#FF786888"/>
        <SolidColorBrush x:Key="ErrorRed" Color="#FFF08888"/>

        <!-- ─── START BUTTON STYLE with hover glow ─── -->
        <Style x:Key="StartBtnStyle" TargetType="Button">
            <Setter Property="Background" Value="#FFC87AA0"/>
            <Setter Property="Foreground" Value="#FF13101C"/>
            <Setter Property="FontWeight" Value="Bold"/>
            <Setter Property="FontSize" Value="16"/>
            <Setter Property="Padding" Value="28,14"/>
            <Setter Property="Cursor" Value="Hand"/>
            <Setter Property="BorderThickness" Value="0"/>
            <Setter Property="Template">
                <Setter.Value>
                    <ControlTemplate TargetType="Button">
                        <Border x:Name="btnBorder"
                                Background="{TemplateBinding Background}"
                                CornerRadius="12"
                                Padding="{TemplateBinding Padding}"
                                BorderThickness="0">
                            <Border.Effect>
                                <DropShadowEffect x:Name="btnShadow"
                                    Color="#FFF0A0C0" BlurRadius="12"
                                    ShadowDepth="0" Opacity="0.3"/>
                            </Border.Effect>
                            <ContentPresenter HorizontalAlignment="Center" VerticalAlignment="Center"/>
                        </Border>
                        <ControlTemplate.Triggers>
                            <Trigger Property="IsMouseOver" Value="True">
                                <Trigger.EnterActions>
                                    <BeginStoryboard>
                                        <Storyboard>
                                            <ColorAnimation
                                                Storyboard.TargetName="btnBorder"
                                                Storyboard.TargetProperty="(Border.Background).(SolidColorBrush.Color)"
                                                To="#FFD88AB0" Duration="0:0:0.25"/>
                                        </Storyboard>
                                    </BeginStoryboard>
                                </Trigger.EnterActions>
                                <Trigger.ExitActions>
                                    <BeginStoryboard>
                                        <Storyboard>
                                            <ColorAnimation
                                                Storyboard.TargetName="btnBorder"
                                                Storyboard.TargetProperty="(Border.Background).(SolidColorBrush.Color)"
                                                To="#FFC87AA0" Duration="0:0:0.25"/>
                                        </Storyboard>
                                    </BeginStoryboard>
                                </Trigger.ExitActions>
                            </Trigger>
                            <Trigger Property="IsPressed" Value="True">
                                <Trigger.EnterActions>
                                    <BeginStoryboard>
                                        <Storyboard>
                                            <ColorAnimation
                                                Storyboard.TargetName="btnBorder"
                                                Storyboard.TargetProperty="(Border.Background).(SolidColorBrush.Color)"
                                                To="#FFB06890" Duration="0:0:0.1"/>
                                        </Storyboard>
                                    </BeginStoryboard>
                                </Trigger.EnterActions>
                            </Trigger>
                            <Trigger Property="IsEnabled" Value="False">
                                <Setter TargetName="btnBorder" Property="Background" Value="#FF3A2848"/>
                                <Setter Property="Foreground" Value="#FF786888"/>
                            </Trigger>
                        </ControlTemplate.Triggers>
                    </ControlTemplate>
                </Setter.Value>
            </Setter>
        </Style>

        <!-- ─── CLEAR BUTTON STYLE ─── -->
        <Style x:Key="ClearBtnStyle" TargetType="Button">
            <Setter Property="Background" Value="#FF211A30"/>
            <Setter Property="Foreground" Value="#FFA898B8"/>
            <Setter Property="FontWeight" Value="SemiBold"/>
            <Setter Property="FontSize" Value="14"/>
            <Setter Property="Padding" Value="24,14"/>
            <Setter Property="Cursor" Value="Hand"/>
            <Setter Property="BorderThickness" Value="1"/>
            <Setter Property="BorderBrush" Value="#FF3A2D50"/>
            <Setter Property="Template">
                <Setter.Value>
                    <ControlTemplate TargetType="Button">
                        <Border x:Name="clrBorder"
                                Background="{TemplateBinding Background}"
                                CornerRadius="12"
                                Padding="{TemplateBinding Padding}"
                                BorderThickness="1"
                                BorderBrush="#FF3A2D50">
                            <ContentPresenter HorizontalAlignment="Center" VerticalAlignment="Center"/>
                        </Border>
                        <ControlTemplate.Triggers>
                            <Trigger Property="IsMouseOver" Value="True">
                                <Trigger.EnterActions>
                                    <BeginStoryboard>
                                        <Storyboard>
                                            <ColorAnimation
                                                Storyboard.TargetName="clrBorder"
                                                Storyboard.TargetProperty="(Border.Background).(SolidColorBrush.Color)"
                                                To="#FF2D2540" Duration="0:0:0.2"/>
                                            <ColorAnimation
                                                Storyboard.TargetName="clrBorder"
                                                Storyboard.TargetProperty="(Border.BorderBrush).(SolidColorBrush.Color)"
                                                To="#FF5A4080" Duration="0:0:0.2"/>
                                        </Storyboard>
                                    </BeginStoryboard>
                                </Trigger.EnterActions>
                                <Trigger.ExitActions>
                                    <BeginStoryboard>
                                        <Storyboard>
                                            <ColorAnimation
                                                Storyboard.TargetName="clrBorder"
                                                Storyboard.TargetProperty="(Border.Background).(SolidColorBrush.Color)"
                                                To="#FF211A30" Duration="0:0:0.2"/>
                                            <ColorAnimation
                                                Storyboard.TargetName="clrBorder"
                                                Storyboard.TargetProperty="(Border.BorderBrush).(SolidColorBrush.Color)"
                                                To="#FF3A2D50" Duration="0:0:0.2"/>
                                        </Storyboard>
                                    </BeginStoryboard>
                                </Trigger.ExitActions>
                            </Trigger>
                        </ControlTemplate.Triggers>
                    </ControlTemplate>
                </Setter.Value>
            </Setter>
        </Style>

    </Window.Resources>

    <!-- ═══ MAIN LAYOUT ═══ -->
    <Grid>

        <!-- Background glow orbs (decorative) -->
        <Ellipse Width="300" Height="300"
                 HorizontalAlignment="Left" VerticalAlignment="Top"
                 Margin="-80,-60,0,0" IsHitTestVisible="False">
            <Ellipse.Fill>
                <RadialGradientBrush>
                    <GradientStop Color="#20F0A0C0" Offset="0"/>
                    <GradientStop Color="#00F0A0C0" Offset="1"/>
                </RadialGradientBrush>
            </Ellipse.Fill>
            <Ellipse.Triggers>
                <EventTrigger RoutedEvent="Loaded">
                    <BeginStoryboard>
                        <Storyboard>
                            <DoubleAnimation
                                Storyboard.TargetProperty="Opacity"
                                From="0.4" To="1" Duration="0:0:3"
                                AutoReverse="True" RepeatBehavior="Forever"/>
                        </Storyboard>
                    </BeginStoryboard>
                </EventTrigger>
            </Ellipse.Triggers>
        </Ellipse>

        <Ellipse Width="260" Height="260"
                 HorizontalAlignment="Right" VerticalAlignment="Bottom"
                 Margin="0,0,-60,-50" IsHitTestVisible="False">
            <Ellipse.Fill>
                <RadialGradientBrush>
                    <GradientStop Color="#18A0B8E8" Offset="0"/>
                    <GradientStop Color="#00A0B8E8" Offset="1"/>
                </RadialGradientBrush>
            </Ellipse.Fill>
            <Ellipse.Triggers>
                <EventTrigger RoutedEvent="Loaded">
                    <BeginStoryboard>
                        <Storyboard>
                            <DoubleAnimation
                                Storyboard.TargetProperty="Opacity"
                                From="1" To="0.4" Duration="0:0:4"
                                AutoReverse="True" RepeatBehavior="Forever"/>
                        </Storyboard>
                    </BeginStoryboard>
                </EventTrigger>
            </Ellipse.Triggers>
        </Ellipse>

        <Ellipse Width="200" Height="200"
                 HorizontalAlignment="Center" VerticalAlignment="Center"
                 Margin="0,-100,0,0" IsHitTestVisible="False" Opacity="0.5">
            <Ellipse.Fill>
                <RadialGradientBrush>
                    <GradientStop Color="#15C8A0E8" Offset="0"/>
                    <GradientStop Color="#00C8A0E8" Offset="1"/>
                </RadialGradientBrush>
            </Ellipse.Fill>
            <Ellipse.Triggers>
                <EventTrigger RoutedEvent="Loaded">
                    <BeginStoryboard>
                        <Storyboard>
                            <DoubleAnimation
                                Storyboard.TargetProperty="Opacity"
                                From="0.3" To="0.7" Duration="0:0:5"
                                AutoReverse="True" RepeatBehavior="Forever"/>
                        </Storyboard>
                    </BeginStoryboard>
                </EventTrigger>
            </Ellipse.Triggers>
        </Ellipse>

        <!-- ═══ MAIN CONTENT ═══ -->
        <Grid x:Name="MainGrid" Margin="24,20,24,18" Opacity="1">
            <Grid.RowDefinitions>
                <RowDefinition Height="Auto"/>
                <RowDefinition Height="Auto"/>
                <RowDefinition Height="Auto"/>
                <RowDefinition Height="*"/>
            </Grid.RowDefinitions>

            <!-- Row 0: Header -->
            <Grid Grid.Row="0" Margin="0,0,0,14">
                <Grid.ColumnDefinitions>
                    <ColumnDefinition Width="*"/>
                    <ColumnDefinition Width="Auto"/>
                </Grid.ColumnDefinitions>

                <StackPanel Orientation="Horizontal" Grid.Column="0" VerticalAlignment="Center">
                    <TextBlock Text="&#x2726;" FontSize="28" Foreground="#FFF0A0C0" Margin="0,0,12,0" VerticalAlignment="Center">
                        <TextBlock.Effect>
                            <DropShadowEffect Color="#FFF0A0C0" BlurRadius="14" ShadowDepth="0" Opacity="0.5"/>
                        </TextBlock.Effect>
                    </TextBlock>
                    <TextBlock Text="CyberSnatcher" FontSize="26" FontWeight="Bold" Foreground="#FFF0A0C0" VerticalAlignment="Center">
                        <TextBlock.Effect>
                            <DropShadowEffect Color="#FFF0A0C0" BlurRadius="8" ShadowDepth="0" Opacity="0.25"/>
                        </TextBlock.Effect>
                    </TextBlock>
                </StackPanel>

                <Border Grid.Column="1" Background="#FF2D2540" CornerRadius="14"
                        Padding="14,6" BorderThickness="1" BorderBrush="#FF3A2D50"
                        VerticalAlignment="Center">
                    <TextBlock Text="TAURI + REACT + RUST" FontSize="11"
                               Foreground="#FF786888" FontWeight="SemiBold"/>
                </Border>
            </Grid>

            <!-- Row 1: Action Bar (all commands, workflow order) -->
            <Border Grid.Row="1" Background="#FF211A30" CornerRadius="12"
                    Padding="12,10" Margin="0,0,0,10"
                    BorderThickness="1" BorderBrush="#FF2D2540">
                <Grid>
                    <Grid.ColumnDefinitions>
                        <ColumnDefinition Width="Auto"/>
                        <ColumnDefinition Width="8"/>
                        <ColumnDefinition Width="Auto"/>
                        <ColumnDefinition Width="8"/>
                        <ColumnDefinition Width="Auto"/>
                        <ColumnDefinition Width="8"/>
                        <ColumnDefinition Width="Auto"/>
                        <ColumnDefinition Width="*"/>
                        <ColumnDefinition Width="Auto"/>
                    </Grid.ColumnDefinitions>

                    <!-- 1. Fix Cargo + Pull (fix conflicts first) -->
                    <Button x:Name="FixCargoBtn" Grid.Column="0"
                            Style="{StaticResource ClearBtnStyle}"
                            Padding="14,9" FontSize="12">
                        <StackPanel Orientation="Horizontal">
                            <TextBlock Text="&#x2692;" FontSize="13" Foreground="#FFE8C0A0"
                                       Margin="0,0,6,0" VerticalAlignment="Center"/>
                            <TextBlock Text="Fix + Pull" VerticalAlignment="Center"/>
                        </StackPanel>
                    </Button>

                    <!-- 2. Pull + Dev (standard workflow) -->
                    <Button x:Name="StartBtn" Grid.Column="2"
                            Style="{StaticResource StartBtnStyle}"
                            Padding="18,9" FontSize="13">
                        <StackPanel Orientation="Horizontal">
                            <TextBlock Text="&#x25B6;" Margin="0,0,7,0"/>
                            <TextBlock Text="Pull + Dev"/>
                        </StackPanel>
                    </Button>

                    <!-- 3. Dev Only (quick start) -->
                    <Button x:Name="DevOnlyBtn" Grid.Column="4"
                            Style="{StaticResource ClearBtnStyle}"
                            Padding="14,9" FontSize="12">
                        <StackPanel Orientation="Horizontal">
                            <TextBlock Text="&#x25B7;" FontSize="14" Foreground="#FFA0B8E8"
                                       Margin="0,0,6,0" VerticalAlignment="Center"/>
                            <TextBlock Text="Dev Only" VerticalAlignment="Center"/>
                        </StackPanel>
                    </Button>

                    <!-- 4. Production Build (final) -->
                    <Button x:Name="ProdBuildBtn" Grid.Column="6"
                            Style="{StaticResource ClearBtnStyle}"
                            Padding="14,9" FontSize="12">
                        <StackPanel Orientation="Horizontal">
                            <TextBlock Text="&#x2605;" FontSize="13" Foreground="#FF88D8A8"
                                       Margin="0,0,6,0" VerticalAlignment="Center"/>
                            <TextBlock Text="Build" VerticalAlignment="Center"/>
                        </StackPanel>
                    </Button>

                    <!-- 5. Clear Log (utility, pushed right) -->
                    <Button x:Name="ClearBtn" Grid.Column="8"
                            Style="{StaticResource ClearBtnStyle}"
                            Padding="14,9" FontSize="11"
                            Content="Clear" Opacity="0.6"/>
                </Grid>
            </Border>

            <!-- Row 2: Status Bar -->
            <Border x:Name="StatusBorder" Grid.Row="2" Background="#FF211A30"
                    CornerRadius="12" Padding="18,12" Margin="0,0,0,10"
                    BorderThickness="1" BorderBrush="#FF2D2540">
                <Grid>
                    <Grid.ColumnDefinitions>
                        <ColumnDefinition Width="Auto"/>
                        <ColumnDefinition Width="*"/>
                        <ColumnDefinition Width="Auto"/>
                    </Grid.ColumnDefinitions>

                    <Ellipse x:Name="StatusDot" Grid.Column="0" Width="12" Height="12"
                             Fill="#FF786888" Margin="0,0,14,0" VerticalAlignment="Center"/>

                    <TextBlock x:Name="StatusText" Grid.Column="1" FontSize="14"
                               Foreground="#FFA898B8" VerticalAlignment="Center">
                        <Run FontWeight="SemiBold" Foreground="#FFF0E8F8" Text="Idle"/>
                        <Run Text=" - waiting to launch"/>
                    </TextBlock>

                    <TextBlock x:Name="SpinnerText" Grid.Column="2" Text="&#x25C6;"
                               FontSize="18" Foreground="#FFC8A0E8"
                               VerticalAlignment="Center" Visibility="Collapsed"
                               RenderTransformOrigin="0.5,0.5">
                        <TextBlock.RenderTransform>
                            <RotateTransform x:Name="SpinnerRotate" Angle="0"/>
                        </TextBlock.RenderTransform>
                    </TextBlock>
                </Grid>
            </Border>

            <!-- Row 3: Log Panel (fills remaining) -->
            <Border Grid.Row="3" Background="#FF18122A" CornerRadius="12"
                    Padding="2" BorderThickness="1" BorderBrush="#FF2A2240">
                <ScrollViewer x:Name="LogScroll" VerticalScrollBarVisibility="Auto"
                              HorizontalScrollBarVisibility="Disabled"
                              Padding="14,12">
                    <RichTextBox x:Name="LogBox" IsReadOnly="True"
                                 Background="Transparent" BorderThickness="0"
                                 FontFamily="Cascadia Code, Consolas, Courier New"
                                 FontSize="14" Foreground="#FFA898B8"
                                 IsDocumentEnabled="True" Padding="0">
                        <RichTextBox.Document>
                            <FlowDocument>
                                <Paragraph TextAlignment="Center" Foreground="#FF786888" FontStyle="Italic">
                                    <Run FontSize="22" Foreground="#FFC8A0E8" Text="&#x2726;"/>
                                    <LineBreak/>
                                    <Run Text="ready when you are~"/>
                                </Paragraph>
                            </FlowDocument>
                        </RichTextBox.Document>
                    </RichTextBox>
                </ScrollViewer>
            </Border>
        </Grid>

        <!-- ═══ SUCCESS FLASH OVERLAY ═══ -->
        <Border x:Name="SuccessFlash" Panel.ZIndex="50"
                IsHitTestVisible="False" Opacity="0">
            <Border.Background>
                <RadialGradientBrush>
                    <GradientStop Color="#3088D8A8" Offset="0"/>
                    <GradientStop Color="#0088D8A8" Offset="1"/>
                </RadialGradientBrush>
            </Border.Background>
        </Border>

    </Grid>
</Window>
"@

# ─── LOAD XAML ──────────────────────────────────────────
$reader = (New-Object System.Xml.XmlNodeReader $xaml)
$window = [Windows.Markup.XamlReader]::Load($reader)

# ─── GET NAMED ELEMENTS ─────────────────────────────────
$MainGrid     = $window.FindName("MainGrid")
$StatusBorder = $window.FindName("StatusBorder")
$StatusDot    = $window.FindName("StatusDot")
$StatusText   = $window.FindName("StatusText")
$SpinnerText  = $window.FindName("SpinnerText")
$SpinnerRotate= $window.FindName("SpinnerRotate")
$LogBox       = $window.FindName("LogBox")
$LogScroll    = $window.FindName("LogScroll")
$StartBtn     = $window.FindName("StartBtn")
$ClearBtn     = $window.FindName("ClearBtn")
$FixCargoBtn  = $window.FindName("FixCargoBtn")
$DevOnlyBtn   = $window.FindName("DevOnlyBtn")
$ProdBuildBtn = $window.FindName("ProdBuildBtn")
$SuccessFlash = $window.FindName("SuccessFlash")

# ─── ANIMATION HELPERS ──────────────────────────────────
function Start-FadeAnimation {
    param($element, [double]$from, [double]$to, [double]$seconds)
    $anim = New-Object System.Windows.Media.Animation.DoubleAnimation
    $anim.From     = $from
    $anim.To       = $to
    $anim.Duration = [System.Windows.Duration]::new([TimeSpan]::FromSeconds($seconds))
    $anim.EasingFunction = New-Object System.Windows.Media.Animation.CubicEase
    $element.BeginAnimation([System.Windows.UIElement]::OpacityProperty, $anim)
}

function Start-ColorAnimation {
    param($brush, [string]$toColor, [double]$seconds)
    $anim = New-Object System.Windows.Media.Animation.ColorAnimation
    $anim.To       = [System.Windows.Media.ColorConverter]::ConvertFromString($toColor)
    $anim.Duration = [System.Windows.Duration]::new([TimeSpan]::FromSeconds($seconds))
    $brush.BeginAnimation([System.Windows.Media.SolidColorBrush]::ColorProperty, $anim)
}

# Spinner rotation (continuous)
$script:spinnerStoryboard = $null
function Start-Spinner {
    $SpinnerText.Visibility = "Visible"
    $anim = New-Object System.Windows.Media.Animation.DoubleAnimation
    $anim.From = 0; $anim.To = 360
    $anim.Duration = [System.Windows.Duration]::new([TimeSpan]::FromSeconds(1.2))
    $anim.RepeatBehavior = [System.Windows.Media.Animation.RepeatBehavior]::Forever
    $SpinnerRotate.BeginAnimation(
        [System.Windows.Media.RotateTransform]::AngleProperty, $anim)
}

function Stop-Spinner {
    $SpinnerRotate.BeginAnimation(
        [System.Windows.Media.RotateTransform]::AngleProperty, $null)
    $SpinnerText.Visibility = "Collapsed"
}

# ─── LOGGING ────────────────────────────────────────────
function Write-Log {
    param([string]$text, [string]$color = "#FFA898B8", [switch]$bold)

    $window.Dispatcher.Invoke([Action]{
        $doc = $LogBox.Document

        # Remove the welcome message on first real log
        if ($script:firstLog) {
            $doc.Blocks.Clear()
            $script:firstLog = $false
        }

        $para = New-Object System.Windows.Documents.Paragraph
        $para.Margin = [System.Windows.Thickness]::new(0, 1, 0, 1)
        $para.LineHeight = 1

        $run = New-Object System.Windows.Documents.Run($text)
        $run.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString($color)
        if ($bold) { $run.FontWeight = "Bold" }
        $para.Inlines.Add($run)
        $doc.Blocks.Add($para)

        # Auto-scroll to bottom
        $LogScroll.ScrollToEnd()
    })
}
$script:firstLog = $true

function Clear-Log {
    $LogBox.Document.Blocks.Clear()
    $script:firstLog = $true
    $para = New-Object System.Windows.Documents.Paragraph
    $para.TextAlignment = "Center"
    $para.Foreground = [System.Windows.Media.Brushes]::Gray

    $starRun = New-Object System.Windows.Documents.Run([char]0x2726)
    $starRun.FontSize = 22
    $starRun.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFC8A0E8")
    $para.Inlines.Add($starRun)
    $para.Inlines.Add((New-Object System.Windows.Documents.LineBreak))

    $textRun = New-Object System.Windows.Documents.Run("ready when you are~")
    $textRun.FontStyle = "Italic"
    $textRun.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FF786888")
    $para.Inlines.Add($textRun)

    $LogBox.Document.Blocks.Add($para)
}

# ─── STATUS ─────────────────────────────────────────────
function Set-Status {
    param([string]$state, [string]$detail)

    $window.Dispatcher.Invoke([Action]{
        $dotBrush = $StatusDot.Fill
        $borderBrush = $StatusBorder.BorderBrush

        switch ($state) {
            "idle" {
                Start-ColorAnimation $dotBrush "#FF786888" 0.4
                Start-ColorAnimation $borderBrush "#FF2D2540" 0.4
                $StatusText.Inlines.Clear()
                $r1 = New-Object System.Windows.Documents.Run("Idle")
                $r1.FontWeight = "SemiBold"; $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFF0E8F8")
                $r2 = New-Object System.Windows.Documents.Run(" - waiting to launch")
                $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                $StatusText.Inlines.Add($r1); $StatusText.Inlines.Add($r2)
                Stop-Spinner
            }
            "running" {
                Start-ColorAnimation $dotBrush "#FFC8A0E8" 0.4
                Start-ColorAnimation $borderBrush "#FF6A50A0" 0.4
                $StatusText.Inlines.Clear()
                $r1 = New-Object System.Windows.Documents.Run("Running")
                $r1.FontWeight = "SemiBold"; $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFC8A0E8")
                $r2 = New-Object System.Windows.Documents.Run(" - $detail")
                $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                $StatusText.Inlines.Add($r1); $StatusText.Inlines.Add($r2)
                Start-Spinner
            }
            "success" {
                Start-ColorAnimation $dotBrush "#FF88D8A8" 0.4
                Start-ColorAnimation $borderBrush "#FF4A9868" 0.4
                $StatusText.Inlines.Clear()
                $r1 = New-Object System.Windows.Documents.Run("Done!")
                $r1.FontWeight = "SemiBold"; $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FF88D8A8")
                $r2 = New-Object System.Windows.Documents.Run(" - all commands completed ~")
                $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                $StatusText.Inlines.Add($r1); $StatusText.Inlines.Add($r2)
                Stop-Spinner
            }
            "error" {
                Start-ColorAnimation $dotBrush "#FFF08888" 0.4
                Start-ColorAnimation $borderBrush "#FFA04848" 0.4
                $StatusText.Inlines.Clear()
                $r1 = New-Object System.Windows.Documents.Run("Error")
                $r1.FontWeight = "SemiBold"; $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFF08888")
                $r2 = New-Object System.Windows.Documents.Run(" - $detail")
                $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                $StatusText.Inlines.Add($r1); $StatusText.Inlines.Add($r2)
                Stop-Spinner
            }
        }
    })
}

# ─── SUCCESS FLASH + SOUND ──────────────────────────────
function Show-SuccessFlash {
    $window.Dispatcher.Invoke([Action]{
        # Flash animation
        $fadeIn = New-Object System.Windows.Media.Animation.DoubleAnimation
        $fadeIn.From = 0; $fadeIn.To = 0.6
        $fadeIn.Duration = [System.Windows.Duration]::new([TimeSpan]::FromSeconds(1.5))
        $fadeIn.AutoReverse = $true
        $SuccessFlash.BeginAnimation([System.Windows.UIElement]::OpacityProperty, $fadeIn)

        # Play sound
        try {
            $chime = "C:\Windows\Media\chimes.wav"
            if (Test-Path $chime) {
                $player = New-Object System.Media.SoundPlayer($chime)
                $player.Play()
            }
        } catch { }
    })
}



# ─── COMMAND EXECUTION (background runspace) ────────────
$StartBtn.Add_Click({
    if ($StartBtn.IsEnabled -eq $false) { return }
    $StartBtn.IsEnabled = $false
    $FixCargoBtn.IsEnabled = $false
    $DevOnlyBtn.IsEnabled = $false
    $ProdBuildBtn.IsEnabled = $false
    Clear-Log
    $script:firstLog = $false
    Set-Status "running" "starting up..."

    # Run commands in a background runspace
    $runspace = [RunspaceFactory]::CreateRunspace()
    $runspace.ApartmentState = "STA"
    $runspace.Open()
    $runspace.SessionStateProxy.SetVariable("window", $window)
    $runspace.SessionStateProxy.SetVariable("WorkDir", $WorkDir)
    $runspace.SessionStateProxy.SetVariable("LogBox", $LogBox)
    $runspace.SessionStateProxy.SetVariable("LogScroll", $LogScroll)
    $runspace.SessionStateProxy.SetVariable("StatusDot", $StatusDot)
    $runspace.SessionStateProxy.SetVariable("StatusBorder", $StatusBorder)
    $runspace.SessionStateProxy.SetVariable("StatusText", $StatusText)
    $runspace.SessionStateProxy.SetVariable("SpinnerText", $SpinnerText)
    $runspace.SessionStateProxy.SetVariable("SpinnerRotate", $SpinnerRotate)
    $runspace.SessionStateProxy.SetVariable("StartBtn", $StartBtn)
    $runspace.SessionStateProxy.SetVariable("FixCargoBtn", $FixCargoBtn)
    $runspace.SessionStateProxy.SetVariable("DevOnlyBtn", $DevOnlyBtn)
    $runspace.SessionStateProxy.SetVariable("ProdBuildBtn", $ProdBuildBtn)
    $runspace.SessionStateProxy.SetVariable("SuccessFlash", $SuccessFlash)

    $ps = [PowerShell]::Create()
    $ps.Runspace = $runspace
    [void]$ps.AddScript({
        # Helper: log from background thread
        function BgLog($text, $color) {
            if (-not $color) { $color = "#FFA898B8" }
            $window.Dispatcher.Invoke([Action]{
                $doc = $LogBox.Document
                $para = New-Object System.Windows.Documents.Paragraph
                $para.Margin = [System.Windows.Thickness]::new(0, 2, 0, 2)
                $run = New-Object System.Windows.Documents.Run($text)
                $run.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString($color)
                if ($color -eq "#FFC8A0E8" -or $color -eq "#FF88D8A8" -or $color -eq "#FFF0A0C0") {
                    $run.FontWeight = "SemiBold"
                }
                $para.Inlines.Add($run)
                $doc.Blocks.Add($para)
                $LogScroll.ScrollToEnd()
            })
        }

        function BgStatus($state, $detail) {
            $window.Dispatcher.Invoke([Action]{
                $dotBrush = $StatusDot.Fill
                $borderBrush = $StatusBorder.BorderBrush

                $ca = { param($brush, $toHex, $sec)
                    $a = New-Object System.Windows.Media.Animation.ColorAnimation
                    $a.To = [System.Windows.Media.ColorConverter]::ConvertFromString($toHex)
                    $a.Duration = [System.Windows.Duration]::new([TimeSpan]::FromSeconds($sec))
                    $brush.BeginAnimation([System.Windows.Media.SolidColorBrush]::ColorProperty, $a)
                }

                $StatusText.Inlines.Clear()
                switch ($state) {
                    "running" {
                        & $ca $dotBrush "#FFC8A0E8" 0.4
                        & $ca $borderBrush "#FF6A50A0" 0.4
                        $r1 = New-Object System.Windows.Documents.Run("Running")
                        $r1.FontWeight = "SemiBold"
                        $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFC8A0E8")
                        $StatusText.Inlines.Add($r1)
                        $r2 = New-Object System.Windows.Documents.Run(" - $detail")
                        $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                        $StatusText.Inlines.Add($r2)
                    }
                    "success" {
                        & $ca $dotBrush "#FF88D8A8" 0.4
                        & $ca $borderBrush "#FF4A9868" 0.4
                        $SpinnerText.Visibility = "Collapsed"
                        $r1 = New-Object System.Windows.Documents.Run("Done!")
                        $r1.FontWeight = "SemiBold"
                        $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FF88D8A8")
                        $StatusText.Inlines.Add($r1)
                        $r2 = New-Object System.Windows.Documents.Run(" - all commands completed ~")
                        $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                        $StatusText.Inlines.Add($r2)
                    }
                    "error" {
                        & $ca $dotBrush "#FFF08888" 0.4
                        & $ca $borderBrush "#FFA04848" 0.4
                        $SpinnerText.Visibility = "Collapsed"
                        $r1 = New-Object System.Windows.Documents.Run("Error")
                        $r1.FontWeight = "SemiBold"
                        $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFF08888")
                        $StatusText.Inlines.Add($r1)
                        $r2 = New-Object System.Windows.Documents.Run(" - $detail")
                        $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                        $StatusText.Inlines.Add($r2)
                    }
                }
            })
        }

        BgLog "--- CyberSnatcher Dev Launcher ---" "#FFF0A0C0"
        BgLog "Working dir: $WorkDir" "#FFF0A0C0"
        BgLog " " "#FF18122A"

        # ── Step 1: clone or pull ──
        $isGitRepo = Test-Path (Join-Path $WorkDir ".git")

        if (-not (Test-Path $WorkDir) -or -not $isGitRepo) {
            BgStatus "running" "cloning repo..."
            BgLog "> git clone https://github.com/shaa123/CyberSnatcher.git" "#FFC8A0E8"
            BgLog "Cloning for the first time, hang tight..." "#FFA898B8"

            # Make sure parent dir exists
            $parent = Split-Path $WorkDir -Parent
            if (-not (Test-Path $parent)) { New-Item -ItemType Directory -Path $parent -Force | Out-Null }

            try {
                $proc = New-Object System.Diagnostics.Process
                $proc.StartInfo.FileName = "cmd.exe"
                $proc.StartInfo.Arguments = "/c cd /d `"$WorkDir`" && git init && git remote add origin https://github.com/shaa123/CyberSnatcher.git && git fetch origin && git reset --hard origin/main 2>&1"
                $proc.StartInfo.UseShellExecute = $false
                $proc.StartInfo.RedirectStandardOutput = $true
                $proc.StartInfo.RedirectStandardError = $true
                $proc.StartInfo.CreateNoWindow = $true
                $proc.Start() | Out-Null

                while (-not $proc.StandardOutput.EndOfStream) {
                    $line = $proc.StandardOutput.ReadLine()
                    if ($line -match "error") { BgLog $line "#FFF08888" }
                    elseif ($line -match "Cloning|done|complete") { BgLog $line "#FF88D8A8" }
                    else { BgLog $line "#FFA898B8" }
                }
                $proc.WaitForExit()

                if ($proc.ExitCode -ne 0) {
                    BgLog "git clone failed with exit code $($proc.ExitCode)" "#FFF08888"
                    BgStatus "error" "git clone failed"
                    $window.Dispatcher.Invoke([Action]{ $StartBtn.IsEnabled = $true; $FixCargoBtn.IsEnabled = $true; $DevOnlyBtn.IsEnabled = $true; $ProdBuildBtn.IsEnabled = $true })
                    return
                }
                BgLog "Cloned successfully!" "#FF88D8A8"
            } catch {
                BgLog "ERROR: $_" "#FFF08888"
                BgStatus "error" "$_"
                $window.Dispatcher.Invoke([Action]{ $StartBtn.IsEnabled = $true; $FixCargoBtn.IsEnabled = $true; $DevOnlyBtn.IsEnabled = $true; $ProdBuildBtn.IsEnabled = $true })
                return
            }
        } else {
            BgStatus "running" "git pull origin main"
            BgLog "> git pull origin main" "#FFC8A0E8"

            try {
                $proc = New-Object System.Diagnostics.Process
                $proc.StartInfo.FileName = "cmd.exe"
                $proc.StartInfo.Arguments = "/c cd /d `"$WorkDir`" && git pull origin main 2>&1"
                $proc.StartInfo.UseShellExecute = $false
                $proc.StartInfo.RedirectStandardOutput = $true
                $proc.StartInfo.RedirectStandardError = $true
                $proc.StartInfo.CreateNoWindow = $true
                $proc.StartInfo.WorkingDirectory = $WorkDir
                $proc.Start() | Out-Null

                while (-not $proc.StandardOutput.EndOfStream) {
                    $line = $proc.StandardOutput.ReadLine()
                    if ($line -match "error" -and $line -notmatch "0 error") { BgLog $line "#FFF08888" }
                    elseif ($line -match "warn") { BgLog $line "#FFE8C878" }
                    elseif ($line -match "up to date|already|success|done") { BgLog $line "#FF88D8A8" }
                    else { BgLog $line "#FFA898B8" }
                }
                $proc.WaitForExit()

                if ($proc.ExitCode -ne 0) {
                    BgLog "git pull failed with exit code $($proc.ExitCode)" "#FFF08888"
                    BgStatus "error" "git pull failed"
                    $window.Dispatcher.Invoke([Action]{ $StartBtn.IsEnabled = $true; $FixCargoBtn.IsEnabled = $true; $DevOnlyBtn.IsEnabled = $true; $ProdBuildBtn.IsEnabled = $true })
                    return
                }
                BgLog "Completed successfully." "#FF88D8A8"
            } catch {
                BgLog "ERROR: $_" "#FFF08888"
                BgStatus "error" "$_"
                $window.Dispatcher.Invoke([Action]{ $StartBtn.IsEnabled = $true; $FixCargoBtn.IsEnabled = $true; $DevOnlyBtn.IsEnabled = $true; $ProdBuildBtn.IsEnabled = $true })
                return
            }
        }

        BgLog " " "#FF18122A"

        # ── Step 2: npm run tauri dev (detached) ──
        BgStatus "running" "launching tauri dev server..."
        BgLog "> npm run tauri dev" "#FFC8A0E8"
        BgLog "Launching dev server in new window..." "#FFA898B8"

        try {
            $psi = New-Object System.Diagnostics.ProcessStartInfo
            $psi.FileName = "cmd.exe"
            $psi.Arguments = "/k cd /d `"$WorkDir`" && npm run tauri dev"
            $psi.UseShellExecute = $true
            $psi.WorkingDirectory = $WorkDir
            [System.Diagnostics.Process]::Start($psi) | Out-Null

            BgLog "Dev server launched successfully!" "#FF88D8A8"
            BgLog " " "#FF18122A"
            BgLog "~ All done! The app should open shortly ~" "#FF88D8A8"
            BgStatus "success" ""

            # Success flash + sound
            $window.Dispatcher.Invoke([Action]{
                $fadeIn = New-Object System.Windows.Media.Animation.DoubleAnimation
                $fadeIn.From = 0; $fadeIn.To = 0.5
                $fadeIn.Duration = [System.Windows.Duration]::new([TimeSpan]::FromSeconds(1.5))
                $fadeIn.AutoReverse = $true
                $SuccessFlash.BeginAnimation([System.Windows.UIElement]::OpacityProperty, $fadeIn)
                try {
                    $chime = "C:\Windows\Media\chimes.wav"
                    if (Test-Path $chime) {
                        (New-Object System.Media.SoundPlayer($chime)).Play()
                    }
                } catch { }
                $StartBtn.IsEnabled = $true
                $FixCargoBtn.IsEnabled = $true
                $DevOnlyBtn.IsEnabled = $true
                $ProdBuildBtn.IsEnabled = $true
            })
        } catch {
            BgLog "ERROR: $_" "#FFF08888"
            BgStatus "error" "$_"
            $window.Dispatcher.Invoke([Action]{ $StartBtn.IsEnabled = $true; $FixCargoBtn.IsEnabled = $true; $DevOnlyBtn.IsEnabled = $true; $ProdBuildBtn.IsEnabled = $true })
        }
    })

    $ps.BeginInvoke() | Out-Null
})

# ─── CLEAR BUTTON ──────────────────────────────────────
$ClearBtn.Add_Click({
    Clear-Log
    Set-Status "idle"
})

# ─── FIX CARGO BUTTON ─────────────────────────────────
$FixCargoBtn.Add_Click({
    if ($StartBtn.IsEnabled -eq $false) { return }
    $StartBtn.IsEnabled = $false
    $FixCargoBtn.IsEnabled = $false
    $DevOnlyBtn.IsEnabled = $false
    $ProdBuildBtn.IsEnabled = $false
    Clear-Log
    $script:firstLog = $false
    Set-Status "running" "fixing Cargo.toml..."

    $runspace = [RunspaceFactory]::CreateRunspace()
    $runspace.ApartmentState = "STA"
    $runspace.Open()
    foreach ($v in @("window","WorkDir","LogBox","LogScroll","StatusDot","StatusBorder",
        "StatusText","SpinnerText","SpinnerRotate","StartBtn","FixCargoBtn",
        "DevOnlyBtn","ProdBuildBtn","SuccessFlash")) {
        $runspace.SessionStateProxy.SetVariable($v, (Get-Variable $v).Value)
    }

    $ps = [PowerShell]::Create()
    $ps.Runspace = $runspace
    [void]$ps.AddScript({
        function BgLog($text, $color) {
            if (-not $color) { $color = "#FFA898B8" }
            $window.Dispatcher.Invoke([Action]{
                $doc = $LogBox.Document
                $para = New-Object System.Windows.Documents.Paragraph
                $para.Margin = [System.Windows.Thickness]::new(0, 2, 0, 2)
                $run = New-Object System.Windows.Documents.Run($text)
                $run.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString($color)
                if ($color -eq "#FFC8A0E8" -or $color -eq "#FF88D8A8" -or $color -eq "#FFF0A0C0") {
                    $run.FontWeight = "SemiBold"
                }
                $para.Inlines.Add($run)
                $doc.Blocks.Add($para)
                $LogScroll.ScrollToEnd()
            })
        }

        function BgStatus($state, $detail) {
            $window.Dispatcher.Invoke([Action]{
                $dotBrush = $StatusDot.Fill
                $borderBrush = $StatusBorder.BorderBrush
                $ca = { param($brush, $toHex, $sec)
                    $a = New-Object System.Windows.Media.Animation.ColorAnimation
                    $a.To = [System.Windows.Media.ColorConverter]::ConvertFromString($toHex)
                    $a.Duration = [System.Windows.Duration]::new([TimeSpan]::FromSeconds($sec))
                    $brush.BeginAnimation([System.Windows.Media.SolidColorBrush]::ColorProperty, $a)
                }
                $StatusText.Inlines.Clear()
                switch ($state) {
                    "running" {
                        & $ca $dotBrush "#FFC8A0E8" 0.4
                        & $ca $borderBrush "#FF6A50A0" 0.4
                        $r1 = New-Object System.Windows.Documents.Run("Running")
                        $r1.FontWeight = "SemiBold"
                        $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFC8A0E8")
                        $StatusText.Inlines.Add($r1)
                        $r2 = New-Object System.Windows.Documents.Run(" - $detail")
                        $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                        $StatusText.Inlines.Add($r2)
                    }
                    "success" {
                        & $ca $dotBrush "#FF88D8A8" 0.4
                        & $ca $borderBrush "#FF4A9868" 0.4
                        $SpinnerText.Visibility = "Collapsed"
                        $r1 = New-Object System.Windows.Documents.Run("Done!")
                        $r1.FontWeight = "SemiBold"
                        $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FF88D8A8")
                        $StatusText.Inlines.Add($r1)
                        $r2 = New-Object System.Windows.Documents.Run(" - all commands completed ~")
                        $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                        $StatusText.Inlines.Add($r2)
                    }
                    "error" {
                        & $ca $dotBrush "#FFF08888" 0.4
                        & $ca $borderBrush "#FFA04848" 0.4
                        $SpinnerText.Visibility = "Collapsed"
                        $r1 = New-Object System.Windows.Documents.Run("Error")
                        $r1.FontWeight = "SemiBold"
                        $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFF08888")
                        $StatusText.Inlines.Add($r1)
                        $r2 = New-Object System.Windows.Documents.Run(" - $detail")
                        $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                        $StatusText.Inlines.Add($r2)
                    }
                }
            })
        }

        function RunCmd($display, $cmd) {
            BgStatus "running" $display
            BgLog "> $display" "#FFC8A0E8"
            try {
                $proc = New-Object System.Diagnostics.Process
                $proc.StartInfo.FileName = "cmd.exe"
                $proc.StartInfo.Arguments = "/c cd /d `"$WorkDir`" && $cmd 2>&1"
                $proc.StartInfo.UseShellExecute = $false
                $proc.StartInfo.RedirectStandardOutput = $true
                $proc.StartInfo.RedirectStandardError = $true
                $proc.StartInfo.CreateNoWindow = $true
                $proc.StartInfo.WorkingDirectory = $WorkDir
                $proc.Start() | Out-Null
                while (-not $proc.StandardOutput.EndOfStream) {
                    $line = $proc.StandardOutput.ReadLine()
                    if ($line -match "error" -and $line -notmatch "0 error") {
                        BgLog $line "#FFF08888"
                    } elseif ($line -match "warn") {
                        BgLog $line "#FFE8C878"
                    } elseif ($line -match "up to date|already|success|done") {
                        BgLog $line "#FF88D8A8"
                    } else {
                        BgLog $line "#FFA898B8"
                    }
                }
                $proc.WaitForExit()
                if ($proc.ExitCode -ne 0) {
                    BgLog "Command failed with exit code $($proc.ExitCode)" "#FFF08888"
                    return $false
                }
                BgLog "Completed successfully." "#FF88D8A8"
                return $true
            } catch {
                BgLog "ERROR: $_" "#FFF08888"
                return $false
            }
        }

        if (-not (Test-Path $WorkDir)) {
            BgLog "ERROR: Directory not found: $WorkDir" "#FFF08888"
            BgStatus "error" "directory not found"
            $window.Dispatcher.Invoke([Action]{ $StartBtn.IsEnabled = $true; $FixCargoBtn.IsEnabled = $true; $DevOnlyBtn.IsEnabled = $true; $ProdBuildBtn.IsEnabled = $true })
            return
        }

        BgLog "--- Fix Cargo.toml + Pull ---" "#FFF0A0C0"
        BgLog "Working dir: $WorkDir" "#FFF0A0C0"
        BgLog " " "#FF18122A"

        # Step 1: git checkout Cargo.toml
        $ok = RunCmd "git checkout -- src-tauri/Cargo.toml" "git checkout -- src-tauri/Cargo.toml"
        if (-not $ok) {
            BgStatus "error" "checkout failed"
            $window.Dispatcher.Invoke([Action]{ $StartBtn.IsEnabled = $true; $FixCargoBtn.IsEnabled = $true; $DevOnlyBtn.IsEnabled = $true; $ProdBuildBtn.IsEnabled = $true })
            return
        }

        BgLog " " "#FF18122A"

        # Step 2: git pull origin main
        $ok = RunCmd "git pull origin main" "git pull origin main"
        if (-not $ok) {
            BgStatus "error" "git pull failed"
            $window.Dispatcher.Invoke([Action]{ $StartBtn.IsEnabled = $true; $FixCargoBtn.IsEnabled = $true; $DevOnlyBtn.IsEnabled = $true; $ProdBuildBtn.IsEnabled = $true })
            return
        }

        BgLog " " "#FF18122A"
        BgLog "~ Cargo.toml fixed and pulled ~" "#FF88D8A8"
        BgStatus "success" ""

        $window.Dispatcher.Invoke([Action]{
            $fadeIn = New-Object System.Windows.Media.Animation.DoubleAnimation
            $fadeIn.From = 0; $fadeIn.To = 0.5
            $fadeIn.Duration = [System.Windows.Duration]::new([TimeSpan]::FromSeconds(1.5))
            $fadeIn.AutoReverse = $true
            $SuccessFlash.BeginAnimation([System.Windows.UIElement]::OpacityProperty, $fadeIn)
            try {
                $chime = "C:\Windows\Media\chimes.wav"
                if (Test-Path $chime) {
                    (New-Object System.Media.SoundPlayer($chime)).Play()
                }
            } catch { }
            $StartBtn.IsEnabled = $true
            $FixCargoBtn.IsEnabled = $true
            $DevOnlyBtn.IsEnabled = $true
            $ProdBuildBtn.IsEnabled = $true
        })
    })

    $ps.BeginInvoke() | Out-Null
})

# ─── DEV ONLY BUTTON (npm run tauri dev, no git pull) ──
$DevOnlyBtn.Add_Click({
    if ($StartBtn.IsEnabled -eq $false) { return }
    $StartBtn.IsEnabled = $false; $FixCargoBtn.IsEnabled = $false
    $DevOnlyBtn.IsEnabled = $false; $ProdBuildBtn.IsEnabled = $false
    Clear-Log; $script:firstLog = $false
    Set-Status "running" "launching dev server..."

    $runspace = [RunspaceFactory]::CreateRunspace()
    $runspace.ApartmentState = "STA"; $runspace.Open()
    foreach ($v in @("window","WorkDir","LogBox","LogScroll","StatusDot","StatusBorder",
        "StatusText","SpinnerText","SpinnerRotate","StartBtn","FixCargoBtn",
        "DevOnlyBtn","ProdBuildBtn","SuccessFlash")) {
        $runspace.SessionStateProxy.SetVariable($v, (Get-Variable $v).Value)
    }

    $ps = [PowerShell]::Create(); $ps.Runspace = $runspace
    [void]$ps.AddScript({
        function BgLog($text, $color) {
            if (-not $color) { $color = "#FFA898B8" }
            $window.Dispatcher.Invoke([Action]{
                $doc = $LogBox.Document
                $para = New-Object System.Windows.Documents.Paragraph
                $para.Margin = [System.Windows.Thickness]::new(0, 2, 0, 2)
                $run = New-Object System.Windows.Documents.Run($text)
                $run.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString($color)
                if ($color -eq "#FFC8A0E8" -or $color -eq "#FF88D8A8" -or $color -eq "#FFF0A0C0") {
                    $run.FontWeight = "SemiBold"
                }
                $para.Inlines.Add($run); $doc.Blocks.Add($para); $LogScroll.ScrollToEnd()
            })
        }

        function EnableAll {
            $window.Dispatcher.Invoke([Action]{
                $StartBtn.IsEnabled = $true; $FixCargoBtn.IsEnabled = $true; $DevOnlyBtn.IsEnabled = $true; $ProdBuildBtn.IsEnabled = $true
                $DevOnlyBtn.IsEnabled = $true; $ProdBuildBtn.IsEnabled = $true
            })
        }

        if (-not (Test-Path $WorkDir)) {
            BgLog "ERROR: Directory not found: $WorkDir" "#FFF08888"
            EnableAll; return
        }

        BgLog "--- Dev Only (no git pull) ---" "#FFF0A0C0"
        BgLog " " "#FF18122A"
        BgLog "> npm run tauri dev" "#FFC8A0E8"
        BgLog "Launching dev server in new window..." "#FFA898B8"

        try {
            $psi = New-Object System.Diagnostics.ProcessStartInfo
            $psi.FileName = "cmd.exe"
            $psi.Arguments = "/k cd /d `"$WorkDir`" && npm run tauri dev"
            $psi.UseShellExecute = $true; $psi.WorkingDirectory = $WorkDir
            [System.Diagnostics.Process]::Start($psi) | Out-Null
            BgLog "Dev server launched!" "#FF88D8A8"
            BgLog " " "#FF18122A"
            BgLog "~ App should open shortly ~" "#FF88D8A8"

            $window.Dispatcher.Invoke([Action]{
                # Status
                $dotBrush = $StatusDot.Fill; $borderBrush = $StatusBorder.BorderBrush
                $a1 = New-Object System.Windows.Media.Animation.ColorAnimation
                $a1.To = [System.Windows.Media.ColorConverter]::ConvertFromString("#FF88D8A8")
                $a1.Duration = [System.Windows.Duration]::new([TimeSpan]::FromSeconds(0.4))
                $dotBrush.BeginAnimation([System.Windows.Media.SolidColorBrush]::ColorProperty, $a1)
                $a2 = New-Object System.Windows.Media.Animation.ColorAnimation
                $a2.To = [System.Windows.Media.ColorConverter]::ConvertFromString("#FF4A9868")
                $a2.Duration = [System.Windows.Duration]::new([TimeSpan]::FromSeconds(0.4))
                $borderBrush.BeginAnimation([System.Windows.Media.SolidColorBrush]::ColorProperty, $a2)
                $SpinnerText.Visibility = "Collapsed"
                $StatusText.Inlines.Clear()
                $r1 = New-Object System.Windows.Documents.Run("Done!")
                $r1.FontWeight = "SemiBold"
                $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FF88D8A8")
                $StatusText.Inlines.Add($r1)
                $r2 = New-Object System.Windows.Documents.Run(" - dev launched ~")
                $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                $StatusText.Inlines.Add($r2)
                # Flash
                $fadeIn = New-Object System.Windows.Media.Animation.DoubleAnimation
                $fadeIn.From = 0; $fadeIn.To = 0.5
                $fadeIn.Duration = [System.Windows.Duration]::new([TimeSpan]::FromSeconds(1.5))
                $fadeIn.AutoReverse = $true
                $SuccessFlash.BeginAnimation([System.Windows.UIElement]::OpacityProperty, $fadeIn)
                try { if (Test-Path "C:\Windows\Media\chimes.wav") {
                    (New-Object System.Media.SoundPlayer("C:\Windows\Media\chimes.wav")).Play()
                }} catch {}
            })
            EnableAll
        } catch {
            BgLog "ERROR: $_" "#FFF08888"
            EnableAll
        }
    })
    $ps.BeginInvoke() | Out-Null
})

# ─── PRODUCTION BUILD BUTTON (npm run build) ──────────
$ProdBuildBtn.Add_Click({
    if ($StartBtn.IsEnabled -eq $false) { return }
    $StartBtn.IsEnabled = $false; $FixCargoBtn.IsEnabled = $false
    $DevOnlyBtn.IsEnabled = $false; $ProdBuildBtn.IsEnabled = $false
    Clear-Log; $script:firstLog = $false
    Set-Status "running" "production build..."

    $runspace = [RunspaceFactory]::CreateRunspace()
    $runspace.ApartmentState = "STA"; $runspace.Open()
    foreach ($v in @("window","WorkDir","LogBox","LogScroll","StatusDot","StatusBorder",
        "StatusText","SpinnerText","SpinnerRotate","StartBtn","FixCargoBtn",
        "DevOnlyBtn","ProdBuildBtn","SuccessFlash")) {
        $runspace.SessionStateProxy.SetVariable($v, (Get-Variable $v).Value)
    }

    $ps = [PowerShell]::Create(); $ps.Runspace = $runspace
    [void]$ps.AddScript({
        function BgLog($text, $color) {
            if (-not $color) { $color = "#FFA898B8" }
            $window.Dispatcher.Invoke([Action]{
                $doc = $LogBox.Document
                $para = New-Object System.Windows.Documents.Paragraph
                $para.Margin = [System.Windows.Thickness]::new(0, 2, 0, 2)
                $run = New-Object System.Windows.Documents.Run($text)
                $run.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString($color)
                if ($color -eq "#FFC8A0E8" -or $color -eq "#FF88D8A8" -or $color -eq "#FFF0A0C0") {
                    $run.FontWeight = "SemiBold"
                }
                $para.Inlines.Add($run); $doc.Blocks.Add($para); $LogScroll.ScrollToEnd()
            })
        }

        function BgStatus($state, $detail) {
            $window.Dispatcher.Invoke([Action]{
                $dotBrush = $StatusDot.Fill; $borderBrush = $StatusBorder.BorderBrush
                $ca = { param($brush, $toHex, $sec)
                    $a = New-Object System.Windows.Media.Animation.ColorAnimation
                    $a.To = [System.Windows.Media.ColorConverter]::ConvertFromString($toHex)
                    $a.Duration = [System.Windows.Duration]::new([TimeSpan]::FromSeconds($sec))
                    $brush.BeginAnimation([System.Windows.Media.SolidColorBrush]::ColorProperty, $a)
                }
                $StatusText.Inlines.Clear()
                switch ($state) {
                    "running" {
                        & $ca $dotBrush "#FFC8A0E8" 0.4; & $ca $borderBrush "#FF6A50A0" 0.4
                        $r1 = New-Object System.Windows.Documents.Run("Running"); $r1.FontWeight = "SemiBold"
                        $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFC8A0E8")
                        $StatusText.Inlines.Add($r1)
                        $r2 = New-Object System.Windows.Documents.Run(" - $detail")
                        $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                        $StatusText.Inlines.Add($r2)
                    }
                    "success" {
                        & $ca $dotBrush "#FF88D8A8" 0.4; & $ca $borderBrush "#FF4A9868" 0.4
                        $SpinnerText.Visibility = "Collapsed"
                        $r1 = New-Object System.Windows.Documents.Run("Done!"); $r1.FontWeight = "SemiBold"
                        $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FF88D8A8")
                        $StatusText.Inlines.Add($r1)
                        $r2 = New-Object System.Windows.Documents.Run(" - build complete ~")
                        $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                        $StatusText.Inlines.Add($r2)
                    }
                    "error" {
                        & $ca $dotBrush "#FFF08888" 0.4; & $ca $borderBrush "#FFA04848" 0.4
                        $SpinnerText.Visibility = "Collapsed"
                        $r1 = New-Object System.Windows.Documents.Run("Error"); $r1.FontWeight = "SemiBold"
                        $r1.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFF08888")
                        $StatusText.Inlines.Add($r1)
                        $r2 = New-Object System.Windows.Documents.Run(" - $detail")
                        $r2.Foreground = [System.Windows.Media.BrushConverter]::new().ConvertFromString("#FFA898B8")
                        $StatusText.Inlines.Add($r2)
                    }
                }
            })
        }

        function EnableAll {
            $window.Dispatcher.Invoke([Action]{
                $StartBtn.IsEnabled = $true; $FixCargoBtn.IsEnabled = $true; $DevOnlyBtn.IsEnabled = $true; $ProdBuildBtn.IsEnabled = $true
                $DevOnlyBtn.IsEnabled = $true; $ProdBuildBtn.IsEnabled = $true
            })
        }

        if (-not (Test-Path $WorkDir)) {
            BgLog "ERROR: Directory not found: $WorkDir" "#FFF08888"
            BgStatus "error" "directory not found"; EnableAll; return
        }

        BgLog "--- Production Build ---" "#FFF0A0C0"
        BgLog "Working dir: $WorkDir" "#FFF0A0C0"
        BgLog " " "#FF18122A"

        BgStatus "running" "npm run build"
        BgLog "> npm run build" "#FFC8A0E8"
        BgLog "This may take a while..." "#FFA898B8"

        try {
            $proc = New-Object System.Diagnostics.Process
            $proc.StartInfo.FileName = "cmd.exe"
            $proc.StartInfo.Arguments = "/c cd /d `"$WorkDir`" && npm run build 2>&1"
            $proc.StartInfo.UseShellExecute = $false
            $proc.StartInfo.RedirectStandardOutput = $true
            $proc.StartInfo.RedirectStandardError = $true
            $proc.StartInfo.CreateNoWindow = $true
            $proc.StartInfo.WorkingDirectory = $WorkDir
            $proc.Start() | Out-Null
            while (-not $proc.StandardOutput.EndOfStream) {
                $line = $proc.StandardOutput.ReadLine()
                if ($line -match "error" -and $line -notmatch "0 error") {
                    BgLog $line "#FFF08888"
                } elseif ($line -match "warn") {
                    BgLog $line "#FFE8C878"
                } elseif ($line -match "success|done|built|finished") {
                    BgLog $line "#FF88D8A8"
                } else {
                    BgLog $line "#FFA898B8"
                }
            }
            $proc.WaitForExit()
            if ($proc.ExitCode -ne 0) {
                BgLog "Build failed with exit code $($proc.ExitCode)" "#FFF08888"
                BgStatus "error" "build failed (exit $($proc.ExitCode))"
                EnableAll; return
            }
            BgLog " " "#FF18122A"
            BgLog "~ Production build complete! ~" "#FF88D8A8"
            BgStatus "success" ""

            $window.Dispatcher.Invoke([Action]{
                $fadeIn = New-Object System.Windows.Media.Animation.DoubleAnimation
                $fadeIn.From = 0; $fadeIn.To = 0.5
                $fadeIn.Duration = [System.Windows.Duration]::new([TimeSpan]::FromSeconds(1.5))
                $fadeIn.AutoReverse = $true
                $SuccessFlash.BeginAnimation([System.Windows.UIElement]::OpacityProperty, $fadeIn)
                try { if (Test-Path "C:\Windows\Media\chimes.wav") {
                    (New-Object System.Media.SoundPlayer("C:\Windows\Media\chimes.wav")).Play()
                }} catch {}
            })
            EnableAll
        } catch {
            BgLog "ERROR: $_" "#FFF08888"
            BgStatus "error" "$_"; EnableAll
        }
    })
    $ps.BeginInvoke() | Out-Null
})

# ─── SHOW WINDOW ───────────────────────────────────────
[void]$window.ShowDialog()

@echo off
cd /d C:\Users\ItsMe\Downloads\CyberSnatcher

:: First-time setup: init repo and set remote if needed
if not exist ".git" (
    git init
    git remote add origin https://github.com/shaa123/CyberSnatcher.git
    echo src-tauri/target/ > .gitignore
    echo node_modules/ >> .gitignore
    echo dist/ >> .gitignore
)

:: Pull latest from remote first (preserves commits from Claude and GitHub Actions)
git fetch origin main
git checkout main 2>nul || git checkout -b main
git pull origin main --rebase

:: Stage, commit, and push
git add .
git diff --cached --quiet || git commit -m "local update"
git push -u origin main

echo.
echo Done! Check your GitHub repo now :)
pause

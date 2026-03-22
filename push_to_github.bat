@echo off
cd /d C:\Users\ItsMe\Downloads\CyberSnatcher

echo src-tauri/target/ > .gitignore
echo node_modules/ >> .gitignore
echo dist/ >> .gitignore

git init
git add .
git commit -m "initial commit"
git branch -M main
git remote add origin https://github.com/shaa123/CyberSnatcher.git
git push -u origin main --force

echo.
echo Done! Check your GitHub repo now :)
pause

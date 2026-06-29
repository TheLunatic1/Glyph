const fs = require('fs');
const { execSync } = require('child_process');
const path = require('path');

const distPath = path.join(__dirname, 'dist');

if (fs.existsSync(distPath)) {
  let zipName;
  const latestYmlPath = path.join(distPath, 'latest.yml');
  
  if (fs.existsSync(latestYmlPath)) {
    const ymlContent = fs.readFileSync(latestYmlPath, 'utf8');
    const match = ymlContent.match(/^version:\s*([^\s]+)/m);
    if (match && match[1]) {
      zipName = `dist_archive_v${match[1]}.zip`;
    }
  }
  
  if (!zipName) {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    zipName = `dist_archive_${timestamp}.zip`;
  }
  
  const archiveDir = path.join(__dirname, 'OLD VERSION');
  if (!fs.existsSync(archiveDir)) {
    fs.mkdirSync(archiveDir);
  }
  
  const zipPath = path.join(archiveDir, zipName);
  
  console.log(`\n📦 Archiving previous build to OLD VERSION/${zipName}...`);
  try {
    execSync(`powershell -Command "Compress-Archive -Path '${distPath}\\*' -DestinationPath '${zipPath}' -Force"`, { stdio: 'inherit' });
    console.log(`✅ Archive created successfully in OLD VERSION/${zipName}`);
    
    fs.rmSync(distPath, { recursive: true, force: true });
    console.log(`🧹 Cleaned old dist directory.\n`);
  } catch (err) {
    console.error('❌ Failed to archive dist directory. Ensure the folder is not locked by another process.');
    console.error(err.message);
    process.exit(1);
  }
} else {
  console.log('\n🧹 No previous dist folder found. Proceeding to build...\n');
}

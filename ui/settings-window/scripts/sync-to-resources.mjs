import {
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  rmSync,
} from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const projectRoot = path.resolve(__dirname, '..')
const distDir = path.resolve(projectRoot, 'dist')
const resourcesDir = path.resolve(projectRoot, '..', '..', 'resources', 'settings_window')
const appIcoPath = path.resolve(projectRoot, '..', '..', 'assets', 'activity.ico')

if (!existsSync(distDir)) {
  throw new Error(`Build output not found: ${distDir}. Run \"bun run build\" first.`)
}

mkdirSync(resourcesDir, { recursive: true })

for (const entry of readdirSync(resourcesDir)) {
  rmSync(path.join(resourcesDir, entry), { recursive: true, force: true })
}

for (const entry of readdirSync(distDir)) {
  cpSync(path.join(distDir, entry), path.join(resourcesDir, entry), { recursive: true })
}

if (existsSync(appIcoPath)) {
  copyFileSync(appIcoPath, path.join(resourcesDir, 'favicon.ico'))
}

console.log(`Synced ${distDir} -> ${resourcesDir}`)
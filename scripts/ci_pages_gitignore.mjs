import { readdir, readFile, writeFile, stat } from 'node:fs/promises'
import { basename, dirname, join } from 'node:path'
import { exec } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'

const execute = promisify(exec)

const ROOT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..')

const PATH_TO_INDEX_HTML = join(ROOT_DIR, 'index.html')
const PATH_TO_WASM_BIN = join(ROOT_DIR, 'bindings/wasm/bin/')

const notGitignored = (
  await execute('git ls-files --cached --others --exclude-standard')
).stdout
  .toString()
  .trim()
  .split(/\r?\n/g)
  .map(path => join(ROOT_DIR, path))
const gitignore = (await readFile(join(ROOT_DIR, '.gitignore')))
  .toString()
  .trim()
const rootFiles = await readdir(ROOT_DIR)

function isNotExcluded(fullPath) {
  const file = basename(fullPath)

  return (
    fullPath === PATH_TO_INDEX_HTML ||
    (fullPath.startsWith(PATH_TO_WASM_BIN) &&
      (file === 'decancer.wasm' || file === 'decancer.min.js'))
  )
}

async function resolveDirectory(directoryName) {
  const files = await readdir(directoryName)
  const output = []

  void (await Promise.all(
    files
      .map(path => join(directoryName, path))
      .map(async path => {
        const fstat = await stat(path)

        if (fstat.isDirectory()) {
          output.push(...(await resolveDirectory(path)))
        } else if (notGitignored.includes(path) && !isNotExcluded(path)) {
          output.push(path)
        }
      })
  ))

  return output
}

const nonPageFiles = await resolveDirectory(ROOT_DIR)

await writeFile(
  join(ROOT_DIR, '.gitignore'),
  `${gitignore}\n${nonPageFiles.join('\n')}`
)

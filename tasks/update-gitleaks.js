const assert = require('assert')
const path = require('path')
const fs = require('fs')
const https = require('https')
const stream = require('stream')

const URL =
    'https://raw.githubusercontent.com/gitleaks/gitleaks/master/config/gitleaks.toml'
const OUTPATH = path.resolve(
    __dirname,
    '..',
    'crates',
    'key_finder',
    'gitleaks.toml'
)

downloadGitleaksTo(OUTPATH)
    .then(() => {
        console.log(`Downloaded gitleaks.toml to '${OUTPATH}'`)
    })
    .catch(e => {
        console.error(e)
        process.exit(1)
    })

/**
 * @param {string} outpath
 *
 * @returns {Promise<void>}
 */
function downloadGitleaksTo(outpath) {
    assert(
        outpath.endsWith('.toml'),
        'Git leads output file is not a TOML file'
    )

    return new Promise((resolve, reject) => {
        https.get(URL, response => {
            const { statusCode, statusMessage } = response

            if (statusCode != null && statusCode >= 300) {
                return reject(new Error(`${statusCode}: ${statusMessage}`))
            }

            stream.pipeline(response, fs.createWriteStream(outpath), err => {
                if (err) reject(err)
                else resolve()
            })
        })
    })
}

const assert = require('assert')
const fs = require('fs')
// import { promises as fs } from 'fs'
const path = require('path')
// import path from 'path'

function buildBodyRequest({
    facetFilters = undefined,
    facets = [
        'app_answers',
        'app_video_public',
        'batch',
        'demo_day_video_public',
        'highlight_black',
        'highlight_latinx',
        'highlight_women',
        'industries',
        'isHiring',
        'nonprofit',
        'question_answers',
        'regions',
        'subindustry',
        'tags',
        'top_company',
        'top_company_by_revenue',
    ],
    hitsPerPage = 1000,
    maxValuesPerFacet = 1000,
    page = 0,
    query = '',
    ...rest
}) {
    let params = new URLSearchParams()
    if (facetFilters) {
        assert(Array.isArray(facetFilters))
        params.set('facetFilters', JSON.stringify(facetFilters))
    }
    facets?.length &&
        params.set('facets', encodeURIComponent(JSON.stringify(facets)))
    params.set('hitsPerPage', hitsPerPage)
    params.set('maxValuesPerFacet', maxValuesPerFacet)
    params.set('page', page)
    query && params.set('query', query)
    for (const key in rest) {
        params.set(key, encodeURIComponent(rest[key]))
    }
    return {
        indexName: 'YCCompany_production',
        params: params.toString(),
    }
}

/**
 * @param {string | undefined} batch e.g. `W24`. Leave undefined to not filter
 * by batch
 * @param {number} numPages number of search results pages to fetch. Default 1
 * @param {number} page search result start page. Default 0.
 *
 * @returns {AsyncGenerator<Record<string, any>, void, undefined>}
 */
async function* getCompanies(batch, numPages = 1, page = 0) {
    assert(numPages > 0 && page >= 0 && numPages >= page)

    while (page < numPages) {
        const body = JSON.stringify({
            requests: [
                buildBodyRequest({
                    facetFilters: batch ? [[`batch:${batch}`]] : undefined,
                    page,
                    analytics: false,
                    clickAnalytics: false,
                }),
                // buildBodyRequest({
                //     analytics: false,
                //     clickAnalytics: false,
                //     facets: ["batch"],
                //     hitsPerPage: 0,
                // })
            ],
        })

        const res = await fetch(
            'https://45bwzj1sgc-dsn.algolia.net/1/indexes/*/queries?x-algolia-agent=Algolia%20for%20JavaScript%20(3.35.1)%3B%20Browser%3B%20JS%20Helper%20(3.16.1)&x-algolia-application-id=45BWZJ1SGC&x-algolia-api-key=MjBjYjRiMzY0NzdhZWY0NjExY2NhZjYxMGIxYjc2MTAwNWFkNTkwNTc4NjgxYjU0YzFhYTY2ZGQ5OGY5NDMxZnJlc3RyaWN0SW5kaWNlcz0lNUIlMjJZQ0NvbXBhbnlfcHJvZHVjdGlvbiUyMiUyQyUyMllDQ29tcGFueV9CeV9MYXVuY2hfRGF0ZV9wcm9kdWN0aW9uJTIyJTVEJnRhZ0ZpbHRlcnM9JTVCJTIyeWNkY19wdWJsaWMlMjIlNUQmYW5hbHl0aWNzVGFncz0lNUIlMjJ5Y2RjJTIyJTVE',
            {
                headers: {
                    accept: 'application/json',
                    'accept-language': 'en-US,en;q=0.9,la;q=0.8',
                    'cache-control': 'no-cache',
                    'content-type': 'application/x-www-form-urlencoded',
                    pragma: 'no-cache',
                    'sec-ch-ua':
                        '"Google Chrome";v="123", "Not:A-Brand";v="8", "Chromium";v="123"',
                    'sec-ch-ua-mobile': '?0',
                    'sec-ch-ua-platform': '"macOS"',
                    'sec-fetch-dest': 'empty',
                    'sec-fetch-mode': 'cors',
                    'sec-fetch-site': 'cross-site',
                    Referer: 'https://www.ycombinator.com/',
                    'Referrer-Policy': 'strict-origin-when-cross-origin',
                },
                body,
                method: 'POST',
            }
        )
        const data = await res.json()

        const { hits: companies, nbPages: totalPages } = data.results[0]

        assert(companies && Array.isArray(companies))
        yield* companies

        // No more pages to fetch, might as well just stop
        if (page >= totalPages) {
            break
        }
        page++
    }
}
async function main() {
    let outdir = path.join(__dirname, '..', 'tmp')
    await fs.promises.mkdir(outdir, { recursive: true })
    const filePath = path.join(outdir, 'yc-companies.csv')
    const csv = fs.createWriteStream(filePath, {
        flags: 'w',
        encoding: 'utf-8',
    })
    csv.write('Name,Website\n')

    const batches = ['W24', 'S23', 'W23', 'S22', 'W22', 'S21', 'W21']
    let companiesFetched = 0

    for (const batch of batches) {
        console.log('Fetching companies in batch:', batch)
        try {
            for await (const company of getCompanies('W24', 2)) {
                companiesFetched++
                csv.write(`${company.name},${company.website}\n`)
            }
        } catch (e) {
            console.error('Error fetching companies in batch:', e)
        }
    }

    csv.end()
    console.log(`Fetched ${companiesFetched} companies to ${filePath}`)
}

main().catch(e => {
    console.error(e)
    process.exit(1)
})

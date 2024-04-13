const { promises: fs } = require('fs')
// import { promises as fs } from 'fs'
const path = require('path')
// import path from 'path'

async function main() {
    const res = await fetch(
        'https://45bwzj1sgc-2.algolianet.com/1/indexes/*/queries?x-algolia-agent=Algolia%20for%20JavaScript%20(3.35.1)%3B%20Browser%3B%20JS%20Helper%20(3.16.1)&x-algolia-application-id=45BWZJ1SGC&x-algolia-api-key=MjBjYjRiMzY0NzdhZWY0NjExY2NhZjYxMGIxYjc2MTAwNWFkNTkwNTc4NjgxYjU0YzFhYTY2ZGQ5OGY5NDMxZnJlc3RyaWN0SW5kaWNlcz0lNUIlMjJZQ0NvbXBhbnlfcHJvZHVjdGlvbiUyMiUyQyUyMllDQ29tcGFueV9CeV9MYXVuY2hfRGF0ZV9wcm9kdWN0aW9uJTIyJTVEJnRhZ0ZpbHRlcnM9JTVCJTIyeWNkY19wdWJsaWMlMjIlNUQmYW5hbHl0aWNzVGFncz0lNUIlMjJ5Y2RjJTIyJTVE',
        {
            headers: {
                accept: 'application/json',
                'accept-language': 'en-US,en;q=0.9,la;q=0.8',
                'cache-control': 'no-cache',
                'content-type': 'application/x-www-form-urlencoded',
                'cross-origin-embedder-policy': 'require-corp',
                'cross-origin-opener-policy': 'same-origin',
                pragma: 'no-cache',
                'sec-ch-ua':
                    '"Not_A Brand";v="8", "Chromium";v="120", "Google Chrome";v="120"',
                'sec-ch-ua-mobile': '?0',
                'sec-ch-ua-platform': '"macOS"',
                'sec-fetch-dest': 'empty',
                'sec-fetch-mode': 'cors',
                'sec-fetch-site': 'cross-site',
            },
            referrer: 'https://www.ycombinator.com/',
            referrerPolicy: 'strict-origin-when-cross-origin',
            body: '{"requests":[{"indexName":"YCCompany_production","params":"facets=%5B%22app_answers%22%2C%22app_video_public%22%2C%22batch%22%2C%22demo_day_video_public%22%2C%22highlight_black%22%2C%22highlight_latinx%22%2C%22highlight_women%22%2C%22industries%22%2C%22isHiring%22%2C%22nonprofit%22%2C%22question_answers%22%2C%22regions%22%2C%22subindustry%22%2C%22tags%22%2C%22top_company%22%2C%22top_company_by_revenue%22%5D&hitsPerPage=1000&maxValuesPerFacet=1000&page=0&query=&tagFilters="}]}',
            method: 'POST',
            mode: 'cors',
            credentials: 'omit',
        }
    )
    const data = await res.json()
    const websites = data.results[0].hits
        .filter(hit => hit.website)
        .map(hit => [hit.name, hit.website])
    // console.log(websites)
    let filePath = path.join(__dirname, 'yc-companies.csv')

    await fs.writeFile(
        filePath,
        'Name,Website\n' +
            websites.map(([name, website]) => `${name},${website}`).join('\n')
    )
    /*
example hit: 
{
    "id": 271,
    "name": "Airbnb",
    "slug": "airbnb",
    "former_names": [],
    "small_logo_thumb_url": "https://bookface-images.s3.amazonaws.com/small_logos/3e9a0092bee2ccf926e650e59c06503ec6b9ee65.png",
    "website": "http://airbnb.com",
    "all_locations": "San Francisco, CA, USA",
    "long_description": "Founded in August of 2008 and based in San Francisco, California, Airbnb is a trusted community marketplace for people to list, discover, and book unique accommodations around the world — online or from a mobile phone. Whether an apartment for a night, a castle for a week, or a villa for a month, Airbnb connects people to unique travel experiences, at any price point, in more than 33,000 cities and 192 countries. And with world-class customer service and a growing community of users, Airbnb is the easiest way for people to monetize their extra space and showcase it to an audience of millions.  \r\n\r\nNo global movement springs from individuals. It takes an entire team united behind something big. Together, we work hard, we laugh a lot, we brainstorm nonstop, we use hundreds of Post-Its a week, and we give the best high-fives in town. Headquartered in San Francisco, we have satellite offices in Dublin, London, Barcelona, Paris, Milan, Copenhagen, Berlin, Moscow, São Paolo, Sydney, and Singapore.",
    "one_liner": "Book accommodations around the world.",
    "team_size": 6132,
    "highlight_black": false,
    "highlight_latinx": false,
    "highlight_women": false,
    "industry": "Consumer",
    "subindustry": "Consumer -> Travel, Leisure and Tourism",
    "launched_at": 1326790856,
    "tags": [
        "Marketplace",
        "Travel"
    ],
    "tags_highlighted": [],
    "top_company": true,
    "top_company_by_revenue": true,
    "isHiring": false,
    "nonprofit": false,
    "batch": "W09",
    "status": "Public",
    "industries": [
        "Consumer",
        "Travel, Leisure and Tourism"
    ],
    "regions": [
        "United States of America",
        "America / Canada"
    ],
    "stage": "Growth",
    "app_video_public": false,
    "demo_day_video_public": false,
    "app_answers": null,
    "question_answers": false,
    "objectID": "271",
    "_highlightResult": {
        "name": {
            "value": "Airbnb",
            "matchLevel": "none",
            "matchedWords": []
        },
        "website": {
            "value": "http://airbnb.com",
            "matchLevel": "none",
            "matchedWords": []
        },
        "all_locations": {
            "value": "San Francisco, CA, USA",
            "matchLevel": "none",
            "matchedWords": []
        },
        "long_description": {
            "value": "Founded in August of 2008 and based in San Francisco, California, Airbnb is a trusted community marketplace for people to list, discover, and book unique accommodations around the world — online or from a mobile phone. Whether an apartment for a night, a castle for a week, or a villa for a month, Airbnb connects people to unique travel experiences, at any price point, in more than 33,000 cities and 192 countries. And with world-class customer service and a growing community of users, Airbnb is the easiest way for people to monetize their extra space and showcase it to an audience of millions.  \r\n\r\nNo global movement springs from individuals. It takes an entire team united behind something big. Together, we work hard, we laugh a lot, we brainstorm nonstop, we use hundreds of Post-Its a week, and we give the best high-fives in town. Headquartered in San Francisco, we have satellite offices in Dublin, London, Barcelona, Paris, Milan, Copenhagen, Berlin, Moscow, São Paolo, Sydney, and Singapore.",
            "matchLevel": "none",
            "matchedWords": []
        },
        "one_liner": {
            "value": "Book accommodations around the world.",
            "matchLevel": "none",
            "matchedWords": []
        },
        "tags": [
            {
                "value": "Marketplace",
                "matchLevel": "none",
                "matchedWords": []
            },
            {
                "value": "Travel",
                "matchLevel": "none",
                "matchedWords": []
            }
        ]
    }
}
*/
}

main().catch(e => {
    console.error(e)
    process.exit(1)
})

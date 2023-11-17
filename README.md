# RSS deduplication

## TLDR

Utility for users of RSS feeds and newsreaders like feedly and newsify to remove duplicates from their feeds if
- the same feed appears in multiple categories or key word searches from the same source
- the same feed is reworked in a developing story and published under diffent URL links or GUIDs

## Usage

### Prerequisites

- you read news using rss feeds
- you use a newsreader like feedly or newsify
- you have exported your list of rss feeds as an opml file (see https://docs.feedly.com/article/52-how-can-i-export-my-sources-and-feeds-through-opml)
- you own and control a web server where you can run a Rust binary to continously convert the original rss feeds into deduplicated rss feeds served from your web server (alternatively you can run the Rust binary on another node which has write access to your/a web servers web directory)

### Security recommendation: 
- do not publish the new OPML file with the redirected feeds nor the feeds.json on your webspace

Rationale: both files contain the file names of your feed.rss files. If you hide the feed.rss files you can avoid that
others discover and use your feeds and thus cause web traffic on your web server because the rss feed files contain uuids that are difficult to guess.

### Get help

```
rssdeduper --help

See https://github.com/Bodobolero/rssdeduper/README.md for more information.
To see logging information invoke with
RUST_LOG=info

Usage: rssfeed [OPTIONS]

Options:
      --so <FILE>
          Sets the source OPML filename
          
          [default: ./feedly-source.opml]

      --to <FILE>
          Sets the target OPML filename
          
          [default: ./feedly-target.opml]

      --ff <FILE>
          Sets the target feed file
          
          [default: ./feeds.json]

      --td <DIRECTORY>
          Sets the target directory for rss feeds
          
          [default: /var/www/html/rss/]

      --up <URL>
          Sets the url prefix to be used in the target OPML file
          
          [default: https://www.bodobolero.com/rss/]

      --wt <SECONDS>
          Sets the wait time in seconds between iterations
          
          [default: 60]

      --it <ITERATIONS>
          Sets the maximum number of iterations, default 0 means unlimited
          
          [default: 0]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```


## what this is about

Sometimes the same content appears under different categories in my RSS newsreader.
I want to serve a copy of the feed.rss files on my own webserver to be consumed by my newsreader which are deduplicated.

## Example of duplicate items for Stuttgarter Zeitung

### Schlagzeilen (www.stuttgarter-zeitung.de/schlagzeilen.rss)
```xml
<item>
      <title>Deutschlands Kirchen am Kipppunkt: Religion ist den meisten völlig  egal</title>
      <link>https://www.stuttgarter-zeitung.de/inhalt.deutschlands-kirchen-am-kipppunkt-religion-ist-den-meisten-voellig-egal.c8990bf7-8c6a-435c-9612-1ad7ee9a60ed.html</link>
      <description>&lt;img src="https://www.stuttgarter-zeitung.de/media.imagefile.468a763b-f2bd-4c2d-a26d-5f549b6282d0.thumbnail.jpg" border="0"&gt;&lt;br /&gt;Religiosit&amp;auml;t und Kirchenbindung schwinden schneller als von manchen erwartet und von anderen bef&amp;uuml;rchtet. F&amp;uuml;r die allermeisten, selbst Christen, spielen Glaube und Religion keine Rolle mehr. Geht es f&amp;uuml;r die Kirchen in Deutschland bereits um Sein oder Nichtsein?</description>
      <pubDate>Wed, 15 Nov 2023 06:51:06 GMT</pubDate>
      <guid>https://www.stuttgarter-zeitung.de/inhalt.deutschlands-kirchen-am-kipppunkt-religion-ist-den-meisten-voellig-egal.c8990bf7-8c6a-435c-9612-1ad7ee9a60ed.html</guid>
      <dc:creator>Markus Brauer</dc:creator>
</item>
```

### Nachrichten des Tages (www.stuttgarter-zeitung.de/news.rss)
```xml
<item>
      <title>Deutschlands Kirchen am Kipppunkt: Religion ist den meisten völlig  egal</title>
      <link>https://www.stuttgarter-zeitung.de/inhalt.deutschlands-kirchen-am-kipppunkt-religion-ist-den-meisten-voellig-egal.c8990bf7-8c6a-435c-9612-1ad7ee9a60ed.html</link>
      <description>&lt;img src="https://www.stuttgarter-zeitung.de/media.imagefile.468a763b-f2bd-4c2d-a26d-5f549b6282d0.thumbnail.jpg" border="0"&gt;&lt;br /&gt;Religiosit&amp;auml;t und Kirchenbindung schwinden schneller als von manchen erwartet und von anderen bef&amp;uuml;rchtet. F&amp;uuml;r die allermeisten, selbst Christen, spielen Glaube und Religion keine Rolle mehr. Geht es f&amp;uuml;r die Kirchen in Deutschland bereits um Sein oder Nichtsein?</description>
      <pubDate>Wed, 15 Nov 2023 06:51:06 GMT</pubDate>
      <guid>https://www.stuttgarter-zeitung.de/inhalt.deutschlands-kirchen-am-kipppunkt-religion-ist-den-meisten-voellig-egal.c8990bf7-8c6a-435c-9612-1ad7ee9a60ed.html</guid>
      <dc:creator>Markus Brauer</dc:creator>
    </item>
```
### Conclusions from this example

Both have same guid!  so creating a set of guid would be enough for deduplication, only add to rss if the same guid has not been in the set before.

However, see https://www.w3schools.com/XML/xml_rss.asp that only description, link and title xml elements are mandatora, guid is optional - so we best use the "link" element as the identifier in our Set.

## XML parsing libraries

This somewhat dated blog gives an overview of Rust parsing approaches and libraries https://mainmatter.com/blog/2020/12/31/xml-and-rust/

Since our RSS feeds typically are small docs I can afford, an in-memory DOM-based parsing approach using xmltree.
I also tried minidom but minidom does not allow XML document without namespaces.


## Aging of news

My observation comparing the same feed at two different times is that the XML file changes in the following way:
- new articles are added at the top
- older articles are removed from the bottom
- articles in a custom time interval overlap between new and old
- even for the same, overlapping article there can be changes in sequence of articles and corrections within an article (like fixing typos in the description, or rephrasing due to A/B test on click rates)
- to avoid that our newsreader looses feeds we should only run in an interval that is much less than the newsreader update interval - or we also need to preserve the older feeds.

## Developing story

Some news organizations (e.g.FAZ) update the same story multiple times for example

From newest to oldest

- https://www.faz.net/aktuell/politik/ausland/gaza-stadt-israelische-armee-fuehrt-razzia-in-schifa-klinik-durch-19314690.html
- https://www.faz.net/aktuell/politik/ausland/gaza-stadt-israelische-armee-fuehrt-einsatz-im-schifa-krankenhaus-aus-19314690.html
- https://www.faz.net/aktuell/politik/ausland/israelische-streitkraefte-dringen-in-schifa-krankenhaus-in-gaza-ein-19314690.html
- https://www.faz.net/israelische-streitkraefte-dringen-in-schifa-krankenhaus-in-gaza-ein-19314690.html

All these links redirect to the most current, updated entry.
In this case all elements (GUID, Title, Link) are continuously updated.

Since more content is added throughput these updates as a developing story it makes sometimes sense to re-read the article.
However usually there is a delay between the user checking the news and the author publishing the article, so it is quite likely that at the time of reading the current version already has enoough details of the developing story and re-readding would be a waste of time.

One possible approach would be to deduplicate those based on the number at the end of the URL (which in the example above is always 19315690). I discovered that many RSS sources add an ID to the end of their link URL which can be used as an identifer, e.g.

- -19314690.html (FAZ)
- -66601915?at_medium=RSS&amp;at_campaign=KARANGA (BBC Mundo)
- .f3d6053d-c298-4b83-8e70-d5d6e7e8ed78.html (Stuttgarter Zeitung)

So a generalized approach could be to use an URL parser, remove all parameters (after ?), then match everything that is either .html or a hexadecimal or decimal number with optional hyphens as the ID of the page.
If we don't find an ID of at least length 6 using this approach we use the full URL as identifier.

This approach is not completely failsafe. If two RSS feed domwains use the same numbering scheme we might have a collision of item IDs from different domains. This is why we prefix the ID with the host/domain.
Thus for the FAZ example above our ID would be `www.faz.net19314690`

## URL parsing library for generating ID (extract identifier, domain and remove query parms)

https://rust.helpful.codes/tutorials/How-to-do-URL-Parsing/

https://docs.rs/url/latest/url/

## Algorithm Approach

- so what we want is the following:
    - the overlapping time window within a feed should be preserved (so older items should only be removed after some time), more specifically if we parse the same feed every 5 minutes we do not want to just preserve new articles but also those non-duplicate articles we have parsed before
    - duplicate entries in different feeds should be removed, the order of RSS feeds in the OPML file define in which feed
      duplicates are preserved
    - duplicate entries within the same feed (developing stories) should also be removed

- data structures for deduplication

- HashMap<ID, (channellink, content)>  a map from the item ID (generated from the item link) to a tuple containing the channel link URL and the item XML elements

- in a loop, each 10 minutes

- we purge the HashMap at 0:00:00 each day, so during the day we will not show duplicates of developing stories

- The user must export his list of feeds in an OPML file which is an XML file listing subscribed feeds (I use the OPML file that can be exported from feedly)

- read in a list of RSS feeds from the file

- for each feed:

- download the feed

- determine if the feed has changed using <lastBuildDate> - if it hasn't changed, continue with the next feed

- for each item:

- create the ID for the item

- if the ID is not in the HashMap keys add it to the HashMap and publish the item to the feed

- if the ID is in the HashMap keys and the feed is the same feed as the one in the HashMap value publish the original item (not the new one) to the feed

- if the ID is in the HashMap keys and the feed is different from the one in the HashMap value do not publish the item

- write a feed file to the local filesystem (NOT the web servers /var/www/html) directory, see security notice above

- write an OPMl file containing all redirected feeds to to the local filesystem (NOT the web servers /var/www/html) directory, see security notice above

- the user must import the new OPML file into his newsreader from the local filesystem whenever the source OPML file changed (whenever the user wants to subscribe to new feeds and has exported a new OPML file)


## OPML lifecycle

A user should be able to continue adding and removing subscriptions.

After a subscription was added or removed, the OPML file needs to be exported and the source OPML file be replaced.

The rssdeduper will then create a new target OPML file merging the existing feed translations with the new feeds (if any).

uuids for existing feeds will be preserved.

So the new OPML file can be deployed step by step to the newsreaders on different devices (if not automatically synchonized) - and the existing feeds can still be accessed.

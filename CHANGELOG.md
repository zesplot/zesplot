## [Unreleased]

Big refactoring effort, including optimizations with regards to i/o and better
error handling/messages towards the user.



### New
* --prefixes now accepts gzipped files from CAIDA's pfx2as directly
* new --csv option, explicitly triggering the CSV parser on the address input
  file, allowing specification of column names to be used for metadata (e.g.
  'ttl' or 'mss')


### Changed
* improved performance when creating addresses file (--create-addresses)
* --prefixes now accepts two formats, either two columns ("prefix/len ASN") or
  three columns ("prefix len ASN")


### Deprecated (at least for now)
* --colour-input hw / dns ?



## [0.1.0] aka "IMC18" - 2018-09-20

Many new features and improvements for our IMC paper.
Some of the bigger ones: 

* the hierchical/recursive properties for more specifics.
* support for ZMAP output files as address files
* more and better colouring options
* statistical functions on datapoints (e.g. variance of TTL)
* --create-addresses and --create-prefixes to help creating more fancy plots


## pre 0.1.0 aka MAPRG-IETF101 - 2018-03-28

Version as presented at the MAPRG session in London.

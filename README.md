# Simple VCF Viewer #

The program starts an HTTP server listening on the given port of the given url. When requesting the index page the data folder is listed and the correspondent HTML view created. When selecting a VCF file from the index the user is redirected to an interactive table view of the file with IGV integration and some visual features.  

### Usage ###

```
$ vcfviewer [options] <data_path>
```

```
Options:
    --port=N        Port to listen for HTTP requests [default: 8080]
    --address=H     Address to use for listening for HTTP requests [default: localhost]
```

 `vcfviewer` expect the data folder to have a specific structure:
 
```bash
data
├── Abi-enza crossover
│   ├── another_test.vcf
│   └── somatic.vcf.gz
└── ind232
    ├── rare.vcf.gz
    └── somatic.vcf.gz
 ```
 

### Build ###

`vcfviewer` can be compiled from source using the `cargo` build system. For example:

```
$ cargo build
```
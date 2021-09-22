use bio::{alignment::sparse::hash_kmers, alphabets::dna::revcomp, io::fasta};
use dashmap::DashMap;
use rayon::prelude::*;
use std::{collections::{HashMap, HashSet}, env, error::Error, fs::File, io::Write, str, time::Instant};
use fxhash::{FxHashMap, FxHashSet};

pub struct Config {
    pub kmer_len: usize,
    pub filepath: String,
}

impl Config {
    pub fn new(mut args: env::Args) -> Result<Config, &'static str> {
        args.next();
	
	let kmer_len = match args.next() {
            Some(arg) => arg.parse().unwrap(),
            None => return Err("Problem with k-mer length input"),
        };
        let filepath = args.next().unwrap();
	
        Ok(Config { kmer_len, filepath })
    }
}

pub fn hash_fasta_rec(
    result: &Result<fasta::Record, std::io::Error>,
    k: usize,
) -> FxHashMap<&[u8], usize> {
    let result_data: &fasta::Record = result.as_ref().unwrap();

    let mut new_hashmap = FxHashMap::default();

    for (kmer, kmer_pos) in hash_kmers(result_data.seq(), k) { // rust-bio's hash_kmers function, returns iterator of tuples (&[u8], Vec<u32>), the Vec being a list of indices of positions of kmer. 
        new_hashmap.insert(kmer, kmer_pos.len());
    }
    new_hashmap
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let start = Instant::now();

    let filepath: String = config.filepath;

    let k: usize = config.kmer_len;

    let reader: fasta::Reader<std::io::BufReader<File>> =
        fasta::Reader::from_file(&filepath).unwrap();

    let fasta_records: Vec<Result<fasta::Record, std::io::Error>> = reader.records().collect();

    let mut hash_vec: Vec<FxHashMap<&[u8], usize>> = fasta_records
        .par_iter()
        .map(|result| hash_fasta_rec(result, k))
        .collect();
    
    let hash_duration = start.elapsed();

    // merging hashmaps
    //eprintln!("length of hash_vec now: {}", hash_vec.len());
    
    let mut hash_len_vec = FxHashSet::default(); // create set of number of kmers 
    
    for h in &hash_vec {
	hash_len_vec.insert(h.len());
    }
    //eprintln!("hashmap lengths: {:?}", hash_len_vec);

    let longest_len = hash_len_vec.iter().max().unwrap();
    
    let i = &hash_vec.iter().position(|h| h.len() == *longest_len).unwrap();

    let mut final_hash: DashMap<&[u8], usize> = hash_vec.remove(*i);

    //eprintln!("this is the hash we're basing off: {:?}", final_hash);

    //eprintln!("length of hash_vec post removal: {}", hash_vec.len());

    hash_vec.par_iter().for_each(|h| {
        for (kmer, freq) in h {
            if final_hash.contains_key(kmer) {
		*final_hash.get_mut(kmer).unwrap() + final_hash[kmer];
            } else {
                final_hash.insert(kmer, *freq);
            }
        }
    });

    let uniq_duration = start.elapsed();

    

    let stdout_ref = &std::io::stdout();

    final_hash.par_iter().for_each(|(k, f)| {
        let kmer = str::from_utf8(k).unwrap();

        if kmer.contains("N") {
        } else {
            let rvc = revcomp(*k);

            let rvc = str::from_utf8(&rvc).unwrap();

            let mut lck = stdout_ref.lock();

            writeln!(&mut lck, "{}\t{}\t{}", kmer, rvc, f).expect("Couldn't write output");
        }
    });
    let duration = start.elapsed();

    eprintln!(
        "Time elapsed creating hashmaps of all kmers in all sequences: {:?}\n",
        hash_duration
    );
    eprintln!("Time elapsed merging hashmaps: {:?}\n", uniq_duration);

    eprintln!("Time elapsed in runtime: {:?}\n", duration);

    Ok(())
}

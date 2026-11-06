fn main(){ let s=std::env::args().nth(1).unwrap().replace("\\n","\n");
 for e in pulldown_cmark::Parser::new_ext(&s, pulldown_cmark::Options::all()) { println!("{:?}",e);} }

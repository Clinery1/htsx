use s_expression_parser::{
    File as SFile,
};
use clap::{
    Parser,
    Subcommand,
};
use std::{
    fmt::Write,
    fs::{
        read_to_string,
        write as write_file,
    },
};
use html::{
    Item as HtmlItem,
};
use css::{
    Item as CssItem,
};


mod html;
mod css;


#[derive(Subcommand,Debug)]
pub enum SubCommand {
    Lsp,
    Convert {
        #[clap(parse(from_flag),short,long)]
        #[clap(help="Generate human-readable HTML code")]
        pretty:bool,
        names:Vec<String>,
    },
}


#[derive(Parser,Debug)]
pub struct Command {
    #[clap(subcommand)]
    pub subcommand:SubCommand,
}


fn main() {
    let args=Command::parse();
    use SubCommand as SC;
    match args.subcommand {
        SC::Convert{names,pretty}=>names.into_iter().for_each(|name|parse_and_write(&name,pretty)),
        SC::Lsp=>todo!("LSP client"),
    }
}
fn parse_and_write(name:&str,pretty:bool) {
    if name.ends_with(".htsx") {
        let contents=read_to_string(&name).unwrap();
        let file=SFile::parse_file(&contents).unwrap();
        let mut elements:Vec<HtmlItem>=Vec::new();
        for i in file.items.iter() {
            elements.push(i.try_into().unwrap());
        }
        let mut out=String::new();
        if pretty {
            for i in elements.iter() {
                write!(out,"{:#}",i).unwrap();
            }
        } else {
            for i in elements.iter() {
                write!(out,"{}",i).unwrap();
            }
        }
        write_file(format!("{}ml",&name[..name.len()-2]),out).unwrap();
    } else if name.ends_with(".cssx") {
        let contents=read_to_string(&name).unwrap();
        let file=SFile::parse_file(&contents).unwrap();
        let mut elements:Vec<CssItem>=Vec::new();
        for i in file.items.iter() {
            elements.push(i.try_into().unwrap());
        }
        let mut out=String::new();
        for i in elements.iter() {
            // dbg!(i);
            i.into_css(&mut out,0).unwrap();
        }
        write_file(format!("{}",&name[..name.len()-1]),out).unwrap();
    }
}

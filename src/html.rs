use indexmap::IndexMap;
use s_expression_parser::{
    Object,
    Location,
};
use std::{
    fmt::{
        Write,
        Error as FmtError,
        Display,
        Formatter,
    },
};


#[derive(Debug)]
pub enum ErrorType {
    InvalidAttribute,
    InvalidTagName,
    EmptyList,
}
pub enum Item<'input> {
    Tag {
        name:&'input str,
        attributes:IndexMap<&'input str,Option<&'input str>>,
        inner:Vec<Self>,
    },
    EmptyTag {
        name:&'input str,
        attributes:IndexMap<&'input str,Option<&'input str>>,
    },
    Text(&'input str),
}
impl<'input> Item<'input> {
    fn create_empty_tag(name:&'input str,attrs:&'input [Object<'input>])->Result<Self,Error> {
        let mut attributes=IndexMap::new();
        for item in attrs {
            match item {
                Object::Ident(_,s,_)|Object::Number(_,s,_)=>{attributes.insert(*s,None);},
                Object::String(_,s,_)=>{attributes.insert(s.as_str(),None);},
                Object::List(attr_start,items,attr_end)=>{
                    match items.as_slice() {
                        [Object::Ident(_,name,_),Object::Ident(_,data,_)|Object::Number(_,data,_)]=>{
                            attributes.insert(*name,Some(*data));
                        },
                        [Object::Ident(_,name,_),Object::String(_,data,_)]=>{
                            attributes.insert(*name,Some(data.as_str()));
                        },
                        _=>{
                            dbg!(items);
                            return Err(Error{start:*attr_start,end:*attr_end,err_type:ErrorType::InvalidAttribute});
                        },
                    }
                },
            }
        }
        Ok(Self::EmptyTag{name,attributes})
    }
    fn create_tag(name:&'input str,attrs:&'input [Object<'input>],tags:&'input [Object<'input>])->Result<Self,Error> {
        let mut attributes=IndexMap::new();
        for item in attrs {
            match item {
                Object::Ident(_,s,_)|Object::Number(_,s,_)=>{attributes.insert(*s,None);},
                Object::String(_,s,_)=>{attributes.insert(s.as_str(),None);},
                Object::List(attr_start,items,attr_end)=>{
                    match items.as_slice() {
                        [Object::Ident(_,name,_),Object::Ident(_,data,_)|Object::Number(_,data,_)]=>{
                            attributes.insert(*name,Some(*data));
                        },
                        [Object::Ident(_,name,_),Object::String(_,data,_)]=>{
                            attributes.insert(*name,Some(data.as_str()));
                        },
                        _=>return Err(Error{start:*attr_start,end:*attr_end,err_type:ErrorType::InvalidAttribute}),
                    }
                },
            }
        }
        let mut inner=Vec::new();
        for i in tags {
            inner.push(i.try_into()?);
        }
        Ok(Self::Tag{name,inner,attributes})
    }
}
impl<'input> Display for Item<'input> {
    fn fmt(&self,f:&mut Formatter)->Result<(),FmtError> {
        if f.alternate() {
            let indent=f.width().unwrap_or(0);
            match self {
                Self::Tag{name,attributes,inner}=>{
                    for _ in 0..indent {
                        write!(f," ")?;
                    }
                    write!(f,"<{}",name)?;
                    for (attribute,maybe_data) in attributes {
                        if let Some(data)=maybe_data {
                            write!(f," {}=\"{}\"",attribute,data)?;
                        } else {
                            write!(f," {}",attribute)?;
                        }
                    }
                    f.write_char('>')?;
                    f.write_char('\n')?;
                    for i in inner {
                        write!(f,"{:#1$}",i,indent+4)?;
                    }
                    for _ in 0..indent {
                        write!(f," ")?;
                    }
                    writeln!(f,"</{}>",name)
                },
                Self::EmptyTag{name,attributes}=>{
                    for _ in 0..indent {
                        write!(f," ")?;
                    }
                    write!(f,"<{}",name)?;
                    for (attribute,maybe_data) in attributes {
                        if let Some(data)=maybe_data {
                            write!(f," {}=\"{}\"",attribute,data)?;
                        } else {
                            write!(f," {}",attribute)?;
                        }
                    }
                    f.write_char('>')?;
                    f.write_char('\n')
                },
                Self::Text(text)=>{
                    for _ in 0..indent {
                        write!(f," ")?;
                    }
                    f.write_str(&text)?;
                    f.write_char('\n')
                },
            }
        } else {
            match self {
                Self::Tag{name,attributes,inner}=>{
                    write!(f,"<{}",name)?;
                    for (attribute,maybe_data) in attributes {
                        if let Some(data)=maybe_data {
                            write!(f," {}=\"{}\"",attribute,data)?;
                        } else {
                            write!(f," {}",attribute)?;
                        }
                    }
                    f.write_char('>')?;
                    for i in inner {
                        i.fmt(f)?;
                    }
                    write!(f,"</{}>",name)
                },
                Self::EmptyTag{name,attributes}=>{
                    write!(f,"<{}",name)?;
                    for (attribute,maybe_data) in attributes {
                        if let Some(data)=maybe_data {
                            write!(f," {}=\"{}\"",attribute,data)?;
                        } else {
                            write!(f," {}",attribute)?;
                        }
                    }
                    f.write_char('>')
                },
                Self::Text(text)=>f.write_str(&text),
            }
        }
    }
}
impl<'input> TryFrom<&'input Object<'input>> for Item<'input> {
    type Error=Error;
    fn try_from(o:&'input Object<'input>)->Result<Self,Self::Error> {
        match o {
            Object::String(_,s,_)=>Ok(Self::Text(s)),
            Object::Ident(_,s,_)|Object::Number(_,s,_)=>Ok(Self::Text(s)),
            Object::List(start,items,end)=>{
                match items.as_slice() {
                    [Object::Ident(_,name,_),attrs@..] if name.starts_with('!')=>Self::create_empty_tag(&name[1..],attrs),
                    [Object::Ident(_,name,_)|Object::Number(_,name,_),rest@..]=>Self::create_tag(name,&[],rest),
                    [Object::String(_,name,_),rest@..]=>Self::create_tag(name.as_str(),&[],rest),
                    [Object::List(tag_start,tag_items,tag_end),rest@..]=>{
                        match tag_items.as_slice() {
                            [Object::Ident(_,name,_),raw_attributes@..]=>{
                                Self::create_tag(name,raw_attributes,rest)
                            },
                            _=>Err(Error{start:*tag_start,end:*tag_end,err_type:ErrorType::InvalidTagName}),
                        }
                    },
                    []=>Err(Error{start:*start,end:*end,err_type:ErrorType::EmptyList}),
                }
            },
        }
    }
}


#[derive(Debug)]
pub struct Error {
    pub start:Location,
    pub end:Location,
    pub err_type:ErrorType,
}

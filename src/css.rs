use s_expression_parser::{
    Object,
    Location,
};
use std::{
    fmt::{
        Result as FmtResult,
        Write,
    },
};


#[derive(Debug)]
pub enum ErrorType {
    ListNotAllowed,
    StringNotAllowed,
    IdentNotAllowed,
    NumberNotAllowed,
    ExpectedSeqOrList,
    ExpectedPercent,
    ExpectedFontValue,
    InvalidAttribute,
    EmptyList,
    // UnknownName,
}
#[derive(Debug)]
pub enum AttributeData<'input> {
    NotImportant(Box<Self>),
    Function {
        name:&'input str,
        args:Vec<Self>,
    },
    List(Vec<Self>),
    Text(&'input str),
    String(&'input str),
}
impl<'input> AttributeData<'input> {
    pub fn into_css<W:Write>(&self,f:&mut W)->FmtResult {
        match self {
            Self::Text(text)=>f.write_str(text),
            Self::String(s)=>{
                f.write_char('"')?;
                f.write_str(s)?;
                f.write_char('"')
            },
            Self::List(items)=>{
                let last=items.len()-1;
                for (i,item) in items.iter().enumerate() {
                    item.into_css(f)?;
                    if i!=last {
                        write!(f," ")?;
                    }
                }
                Ok(())
            },
            Self::NotImportant(item)=>{
                item.into_css(f)?;
                write!(f," !important")
            },
            Self::Function{name,args}=>{
                write!(f,"{}(",name)?;
                let last=args.len()-1;
                for (i,item) in args.iter().enumerate() {
                    item.into_css(f)?;
                    if i!=last {
                        write!(f,", ")?;
                    }
                }
                write!(f,")")
            },
        }
    }
}
impl<'input> TryFrom<&'input Object<'input>> for AttributeData<'input> {
    type Error=Error;
    fn try_from(o:&'input Object<'input>)->Result<Self,Self::Error> {
        match o {
            Object::Ident(_,data,_)|Object::Number(_,data,_)=>Ok(Self::Text(data)),
            Object::List(s,items,e)=>{
                match items.as_slice() {
                    [Object::Ident(_,"!important",_),attr]=>Ok(Self::NotImportant(Box::new(attr.try_into()?))),
                    [Object::Ident(_,"fn",_),Object::Ident(_,name,_),raw_args@..]=>{
                        let mut args=Vec::new();
                        for a in raw_args {
                            args.push(a.try_into()?);
                        }
                        Ok(Self::Function{name,args})
                    },
                    []=>Err(Error{start:*s,end:*e,err_type:ErrorType::EmptyList}),
                    _=>{
                        let mut attrs=Vec::new();
                        for i in items {
                            attrs.push(i.try_into()?);
                        }
                        Ok(Self::List(attrs))
                    },
                }
            },
            Object::String(_,s,_)=>Ok(Self::String(s)),
        }
    }
}
#[derive(Debug,Clone)]
pub enum SelectorType<'input> {
    List(Vec<Self>),
    Sequence(Vec<Self>),
    Class(&'input str),
    Id(&'input str),
    Tag(&'input str),
}
impl<'input> SelectorType<'input> {
    pub fn into_css<W:Write>(&self,f:&mut W)->FmtResult {
        match self {
            Self::List(items)=>{
                let last=items.len()-1;
                for (i,item) in items.iter().enumerate() {
                    item.into_css(f)?;
                    if i!=last {
                        write!(f,", ")?;
                    }
                }
                Ok(())
            },
            Self::Sequence(items)=>{
                let last=items.len()-1;
                for (i,item) in items.iter().enumerate() {
                    item.into_css(f)?;
                    if i!=last {
                        write!(f," ")?;
                    }
                }
                Ok(())
            },
            Self::Class(name)=>write!(f,".{}",name),
            Self::Id(name)=>write!(f,"#{}",name),
            Self::Tag(name)=>write!(f,"{}",name),
        }
    }
}
impl<'input> TryFrom<&'input Object<'input>> for SelectorType<'input> {
    type Error=Error;
    fn try_from(o:&'input Object<'input>)->Result<Self,Self::Error> {
        match o {
            Object::List(start,items,end)=>{
                match items.as_slice() {
                    [Object::Ident(_,"seq",_),rest@..]=>{
                        let mut items=Vec::new();
                        for i in rest {
                            match i {
                                Object::Ident(..)|Object::Number(..)|Object::String(..)=>items.push(Self::try_from(i)?),
                                Object::List(start,_,end)=>return Err(Error{start:*start,end:*end,err_type:ErrorType::ListNotAllowed}),
                            }
                        }
                        Ok(Self::Sequence(items))
                    },
                    [Object::Ident(_,"list",_),rest@..]=>{
                        let mut items=Vec::new();
                        for i in rest {
                            items.push(i.try_into()?);
                        }
                        Ok(Self::List(items))
                    },
                    []=>Err(Error{start:*start,end:*end,err_type:ErrorType::EmptyList}),
                    _=>Err(Error{start:*start,end:*end,err_type:ErrorType::ExpectedSeqOrList}),
                }
            },
            Object::Ident(_,name,_)=>{
                if name.starts_with('#') {
                    Ok(Self::Id(&name[1..]))
                } else if name.starts_with('.') {
                    Ok(Self::Class(&name[1..]))
                } else {
                    Ok(Self::Tag(name))
                }
            },
            Object::Number(s,_,e)=>Err(Error{start:*s,end:*e,err_type:ErrorType::NumberNotAllowed}),
            Object::String(s,_,e)=>Err(Error{start:*s,end:*e,err_type:ErrorType::StringNotAllowed}),
        }
    }
}
#[derive(Debug)]
pub struct Rule<'input> {
    pub selector:SelectorType<'input>,
    pub inner:Vec<(&'input str,AttributeData<'input>)>,
}
impl<'input> Rule<'input> {
    pub fn into_css<W:Write>(&self,f:&mut W,indent:usize)->FmtResult {
        for _ in 0..indent {
            f.write_char(' ')?;
        }
        self.selector.into_css(f)?;
        writeln!(f," {{")?;
        for i in self.inner.iter() {
            for _ in 0..indent+4 {
                f.write_char(' ')?;
            }
            write!(f,"{}: ",i.0)?;
            i.1.into_css(f)?;
            writeln!(f,";")?;
        }
        for _ in 0..indent {
            f.write_char(' ')?;
        }
        writeln!(f,"}}")
    }
}
impl<'input> TryFrom<&'input Object<'input>> for Rule<'input> {
    type Error=Error;
    fn try_from(o:&'input Object<'input>)->Result<Self,Self::Error> {
        match o {
            Object::List(start,items,end)=>{
                match items.as_slice() {
                    [selector,rest@..]=>{
                        let mut inner=Vec::new();
                        for i in rest {
                            match i {
                                Object::List(s,items,e)=>match items.as_slice() {
                                    [Object::Ident(_,name,_),data]=>inner.push((*name,data.try_into()?)),
                                    _=>return Err(Error{start:*s,end:*e,err_type:ErrorType::InvalidAttribute}),
                                },
                                Object::Ident(s,_,e)|Object::Number(s,_,e)|Object::String(s,_,e)=>return Err(Error{start:*s,end:*e,err_type:ErrorType::InvalidAttribute}),
                            }
                        }
                        Ok(Self{selector:selector.try_into()?,inner})
                    },
                    []=>Err(Error{start:*start,end:*end,err_type:ErrorType::EmptyList}),
                }
            },
            Object::Ident(s,_,e)=>Err(Error{start:*s,end:*e,err_type:ErrorType::IdentNotAllowed}),
            Object::Number(s,_,e)=>Err(Error{start:*s,end:*e,err_type:ErrorType::NumberNotAllowed}),
            Object::String(s,_,e)=>Err(Error{start:*s,end:*e,err_type:ErrorType::StringNotAllowed}),
        }
    }
}
#[derive(Debug)]
pub enum MediaQuery<'input> {
    List(Vec<Self>),
    And(Box<[Self;2]>),
    Not(Box<Self>),
    Attribute(&'input str,AttributeData<'input>),
    EmptyAttribute(&'input str),
    Screen,
    Page,
    All,
}
impl<'input> MediaQuery<'input> {
    pub fn into_css<W:Write>(&self,f:&mut W,first:bool)->FmtResult {
        match self {
            Self::List(items)=>{
                if !first {write!(f,"(")?}
                let last=items.len()-1;
                for (i,item) in items.iter().enumerate() {
                    item.into_css(f,false)?;
                    if i!=last {
                        write!(f,", ")?;
                    }
                }
                if !first {write!(f,")")} else {Ok(())}
            },
            Self::And(items)=>{
                if !first {write!(f,"(")?;}
                items[0].into_css(f,false)?;
                write!(f," and ")?;
                items[1].into_css(f,false)?;
                if !first {write!(f,")")} else {Ok(())}
            },
            Self::Not(item)=>{
                if !first {
                    write!(f,"not ")?;
                } else {
                    write!(f,"(not ")?;
                }
                item.into_css(f,false)?;
                write!(f,")")
            },
            Self::EmptyAttribute(attr)=>f.write_str(attr),
            Self::Attribute(name,data)=>{
                write!(f,"({}: ",name)?;
                data.into_css(f)?;
                write!(f,")")
            },
            Self::Screen=>write!(f,"screen"),
            Self::Page=>write!(f,"page"),
            Self::All=>write!(f,"all"),
        }
    }
}
impl<'input> TryFrom<&'input Object<'input>> for MediaQuery<'input> {
    type Error=Error;
    fn try_from(o:&'input Object<'input>)->Result<Self,Self::Error> {
        match o {
            Object::Ident(_,name,_)=>match *name {
                "screen"=>Ok(Self::Screen),
                "all"=>Ok(Self::All),
                "page"=>Ok(Self::Page),
                _=>Ok(Self::EmptyAttribute(name)),
            },
            Object::List(s,items,e)=>{
                match items.as_slice() {
                    [left,Object::Ident(_,"and",_),right]=>Ok(Self::And(Box::new([left.try_into()?,right.try_into()?]))),
                    [Object::Ident(_,"not",_),item]=>Ok(Self::Not(Box::new(item.try_into()?))),
                    [Object::Ident(_,name,_),data]=>Ok(Self::Attribute(name,data.try_into()?)),
                    []=>Err(Error{start:*s,end:*e,err_type:ErrorType::EmptyList}),
                    _=>{
                        let mut list=Vec::new();
                        for i in items {
                            list.push(i.try_into()?);
                        }
                        Ok(Self::List(list))
                    },
                }
            },
            Object::Number(s,_,e)=>Err(Error{start:*s,end:*e,err_type:ErrorType::NumberNotAllowed}),
            Object::String(s,_,e)=>Err(Error{start:*s,end:*e,err_type:ErrorType::StringNotAllowed}),
        }
    }
}
#[derive(Debug)]
pub enum KeyframeRule<'input> {
    Percent {
        percent:&'input str,
        attributes:Vec<(&'input str,AttributeData<'input>)>,
    },
    From(Vec<(&'input str,AttributeData<'input>)>),
    To(Vec<(&'input str,AttributeData<'input>)>),
}
impl<'input> KeyframeRule<'input> {
    pub fn into_css<W:Write>(&self,f:&mut W,indent:usize)->FmtResult {
        for _ in 0..indent {
            f.write_char(' ')?;
        }
        match self {
            Self::Percent{percent,attributes}=>{
                writeln!(f,"{} {{",percent)?;
                for i in attributes {
                    for _ in 0..indent {
                        f.write_char(' ')?;
                    }
                    write!(f,"{}: ",i.0)?;
                    i.1.into_css(f)?;
                    writeln!(f,";")?;
                }
                writeln!(f,"}}")
            },
            Self::From(attributes)=>{
                writeln!(f,"from {{")?;
                for i in attributes {
                    for _ in 0..indent {
                        f.write_char(' ')?;
                    }
                    write!(f,"{}: ",i.0)?;
                    i.1.into_css(f)?;
                    writeln!(f,";")?;
                }
                writeln!(f,"}}")
            },
            Self::To(attributes)=>{
                writeln!(f,"to {{")?;
                for i in attributes {
                    for _ in 0..indent {
                        f.write_char(' ')?;
                    }
                    write!(f,"{}: ",i.0)?;
                    i.1.into_css(f)?;
                    writeln!(f,";")?;
                }
                writeln!(f,"}}")
            },
        }
    }
}
impl<'input> TryFrom<&'input Object<'input>> for KeyframeRule<'input> {
    type Error=Error;
    fn try_from(o:&'input Object<'input>)->Result<Self,Self::Error> {
        fn parse_attrs<'input>(rest:&'input[Object<'input>])->Result<Vec<(&'input str,AttributeData<'input>)>,Error> {
            let mut attrs:Vec<(&'input str,AttributeData<'input>)>=Vec::new();
            for i in rest {
                match i {
                    Object::List(s,items,e)=>match items.as_slice() {
                        [Object::Ident(_,name,_),attr]=>attrs.push((name,attr.try_into()?)),
                        _=>return Err(Error{start:*s,end:*e,err_type:ErrorType::InvalidAttribute}),
                    },
                    Object::Ident(s,_,e)=>return Err(Error{start:*s,end:*e,err_type:ErrorType::IdentNotAllowed}),
                    Object::Number(s,_,e)=>return Err(Error{start:*s,end:*e,err_type:ErrorType::NumberNotAllowed}),
                    Object::String(s,_,e)=>return Err(Error{start:*s,end:*e,err_type:ErrorType::StringNotAllowed}),
                }
            }
            return Ok(attrs);
        }
        match o {
            Object::List(s,items,e)=>match items.as_slice() {
                [Object::Ident(_,"from",_),rest@..]=>Ok(Self::From(parse_attrs(rest)?)),
                [Object::Ident(_,"to",_),rest@..]=>Ok(Self::To(parse_attrs(rest)?)),
                [Object::Ident(_,percent,_),rest@..]=>{
                    if percent.ends_with('%') {
                        Ok(Self::Percent{percent,attributes:parse_attrs(rest)?})
                    } else {
                        Err(Error{start:*s,end:*e,err_type:ErrorType::ExpectedPercent})
                    }
                },
                []=>Err(Error{start:*s,end:*e,err_type:ErrorType::EmptyList}),
                _=>Err(Error{start:*s,end:*e,err_type:ErrorType::InvalidAttribute}),
            },
            Object::Ident(s,_,e)=>Err(Error{start:*s,end:*e,err_type:ErrorType::IdentNotAllowed}),
            Object::Number(s,_,e)=>Err(Error{start:*s,end:*e,err_type:ErrorType::NumberNotAllowed}),
            Object::String(s,_,e)=>Err(Error{start:*s,end:*e,err_type:ErrorType::StringNotAllowed}),
        }
    }
}
#[derive(Debug)]
pub enum FontValue<'input> {
    Url {
        path:&'input str,
        format:&'input str,
    },
    Local {
        path:&'input str,
        format:&'input str,
    },
}
impl<'input> FontValue<'input> {
    pub fn into_css<W:Write>(&self,f:&mut W)->FmtResult {
        match self {
            Self::Url{path,format}=>write!(f,"url(\"{}\") format(\"{}\")",path,format),
            Self::Local{path,format}=>write!(f,"local(\"{}\") format(\"{}\")",path,format),
        }
    }
}
impl<'input> TryFrom<&'input Object<'input>> for FontValue<'input> {
    type Error=Error;
    fn try_from(o:&'input Object<'input>)->Result<Self,Self::Error> {
        match o {
            Object::List(s,items,e)=>match items.as_slice() {
                [Object::Ident(_,"url",_),Object::String(_,path,_),Object::Ident(_,format,_)]=>Ok(Self::Url{path,format}),
                [Object::Ident(_,"local",_),Object::String(_,path,_),Object::Ident(_,format,_)]=>Ok(Self::Local{path,format}),
                []=>Err(Error{start:*s,end:*e,err_type:ErrorType::EmptyList}),
                _=>Err(Error{start:*s,end:*e,err_type:ErrorType::ExpectedFontValue}),
            },
            Object::Ident(s,_,e)=>Err(Error{start:*s,end:*e,err_type:ErrorType::IdentNotAllowed}),
            Object::Number(s,_,e)=>Err(Error{start:*s,end:*e,err_type:ErrorType::NumberNotAllowed}),
            Object::String(s,_,e)=>Err(Error{start:*s,end:*e,err_type:ErrorType::StringNotAllowed}),
        }
    }
}
#[derive(Debug)]
pub enum Item<'input> {
    Rule(Rule<'input>),
    Charset(&'input str),
    FontFace {
        name:&'input str,
        values:Vec<FontValue<'input>>,
    },
    MediaQuery {
        query:MediaQuery<'input>,
        inner:Vec<Self>,
    },
    Import {
        path:AttributeData<'input>,
        query:Option<MediaQuery<'input>>,
    },
    Keyframes {
        name:&'input str,
        rules:Vec<KeyframeRule<'input>>,
    },
    Supports {
        does_support:MediaQuery<'input>,
        inner:Vec<Self>,
    },
    Comment(&'input str),
}
impl<'input> Item<'input> {
    pub fn into_css<W:Write>(&self,f:&mut W,indent:usize)->FmtResult {
        match self {
            Self::Rule(rule)=>rule.into_css(f,indent),
            Self::Charset(data)=>{
                for _ in 0..indent {
                    f.write_char(' ')?;
                }
                writeln!(f,"@charset {};",data)
            },
            Self::FontFace{name,values}=>{
                for _ in 0..indent {
                    f.write_char(' ')?;
                }
                writeln!(f,"@font-face {{")?;
                for _ in 0..indent+4 {
                    f.write_char(' ')?;
                }
                writeln!(f,"font-family: \"{}\";",name)?;
                for _ in 0..indent+4 {
                    f.write_char(' ')?;
                }
                write!(f,"src: ")?;
                let last=values.len()-1;
                for (i,val) in values.iter().enumerate() {
                    val.into_css(f)?;
                    if i!=last {
                        write!(f,", ")?;
                    }
                }
                writeln!(f,";")?;
                for _ in 0..indent {
                    f.write_char(' ')?;
                }
                writeln!(f,"}}")
            },
            Self::MediaQuery{query,inner}=>{
                for _ in 0..indent {
                    f.write_char(' ')?;
                }
                write!(f,"@media ")?;
                query.into_css(f,true)?;
                writeln!(f," {{")?;
                for val in inner.iter() {
                    val.into_css(f,indent+4)?;
                }
                for _ in 0..indent {
                    f.write_char(' ')?;
                }
                writeln!(f,"}}")
            },
            Self::Import{path,query}=>{
                for _ in 0..indent {
                    f.write_char(' ')?;
                }
                write!(f,"@import ")?;
                path.into_css(f)?;
                write!(f," ")?;
                if let Some(query)=query {
                    query.into_css(f,true)?;
                }
                writeln!(f,";")
            },
            Self::Keyframes{name,rules}=>{
                for _ in 0..indent {
                    f.write_char(' ')?;
                }
                writeln!(f,"@keyframes {} {{",name)?;
                for rule in rules {
                    rule.into_css(f,indent+4)?;
                }
                for _ in 0..indent {
                    f.write_char(' ')?;
                }
                writeln!(f,"}}")
            },
            Self::Supports{does_support,inner}=>{
                for _ in 0..indent {
                    f.write_char(' ')?;
                }
                write!(f,"@supports ")?;
                does_support.into_css(f,true)?;
                writeln!(f," {{")?;
                for item in inner {
                    item.into_css(f,indent+4)?;
                }
                for _ in 0..indent {
                    f.write_char(' ')?;
                }
                writeln!(f,"}}")
            },
            Self::Comment(data)=>{
                for _ in 0..indent {
                    f.write_char(' ')?;
                }
                writeln!(f,"/* {} */",data)
            },
        }
    }
}
impl<'input> TryFrom<&'input Object<'input>> for Item<'input> {
    type Error=Error;
    fn try_from(o:&'input Object<'input>)->Result<Self,Self::Error> {
        match o {
            Object::String(_,s,_)=>Ok(Self::Comment(s)),
            Object::Ident(_,s,_)|Object::Number(_,s,_)=>Ok(Self::Comment(s)),
            Object::List(s,items,e)=>{
                match items.as_slice() {
                    [Object::Ident(_,"@media",_),raw_query,rest@..]=>{
                        let query=raw_query.try_into()?;
                        let mut inner=Vec::new();
                        for i in rest {
                            inner.push(i.try_into()?);
                        }
                        Ok(Self::MediaQuery{query,inner})
                    },
                    [Object::Ident(_,"@supports",_),raw_query,rest@..]=>{
                        let does_support=raw_query.try_into()?;
                        let mut inner=Vec::new();
                        for i in rest {
                            inner.push(i.try_into()?);
                        }
                        Ok(Self::Supports{does_support,inner})
                    },
                    [Object::Ident(_,"@charset",_),Object::Ident(_,data,_)|Object::Number(_,data,_)]=>Ok(Self::Charset(data)),
                    [Object::Ident(_,"@charset",_),Object::String(_,data,_)]=>Ok(Self::Charset(data)),
                    [Object::Ident(_,"@import",_),raw_path]=>Ok(Self::Import{path:raw_path.try_into()?,query:None}),
                    [Object::Ident(_,"@import",_),raw_path,raw_query]=>Ok(Self::Import{path:raw_path.try_into()?,query:Some(raw_query.try_into()?)}),
                    [Object::Ident(_,"@keyframes",_),Object::Ident(_,name,_),rest@..]=>{
                        let mut rules=Vec::new();
                        for i in rest {
                            rules.push(i.try_into()?);
                        }
                        Ok(Self::Keyframes{name,rules})
                    },
                    [Object::Ident(_,"@font-face",_),Object::Ident(_,name,_),rest@..]=>{
                        let mut values=Vec::new();
                        for i in rest {
                            values.push(i.try_into()?);
                        }
                        Ok(Self::FontFace{name,values})
                    },
                    [Object::Ident(_,"@font-face",_),Object::String(_,name,_),rest@..]=>{
                        let mut values=Vec::new();
                        for i in rest {
                            values.push(i.try_into()?);
                        }
                        Ok(Self::FontFace{name,values})
                    },
                    []=>Err(Error{start:*s,end:*e,err_type:ErrorType::EmptyList}),
                    _=>Ok(Item::Rule(o.try_into()?)),
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

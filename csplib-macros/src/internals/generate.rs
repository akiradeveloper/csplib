use super::*;

#[derive(Clone)]
enum Interface<T> {
    Writer(T),
    Channel(T),
}

#[derive(Clone)]
struct Struct {
    name: String,
    fields: Vec<Interface<Var>>,
}

struct Chan {
    name: String,
}

pub fn generate(p: ProcessDef) -> String {
    let mut chans = vec![];
    let mut pub_st = Struct {
        name: format!("{}", p.name),
        fields: vec![],
    };
    let mut inn_st = Struct {
        name: format!("{}Inner", p.name),
        fields: vec![],
    };
    for field in p.fields {
        chans.push(Chan {
            name: field.var.name.clone(),
        });
        match field.dir {
            Dir::Input => {
                let for_pub = Interface::Writer(field.var.clone());
                let for_inn = Interface::Channel(field.var);
                pub_st.fields.push(for_pub);
                inn_st.fields.push(for_inn);
            }
            Dir::Output => {
                let for_pub = Interface::Channel(field.var.clone());
                let for_inn = Interface::Writer(field.var);
                pub_st.fields.push(for_pub);
                inn_st.fields.push(for_inn);
            }
        }
    }
    format!(
        "{}{}{}",
        generate_struct(pub_st.clone()),
        generate_struct(inn_st.clone()),
        generate_factory(chans, pub_st, inn_st),
    )
}

fn generate_struct(st: Struct) -> String {
    let mut fields = vec![];
    for field in st.fields {
        match field {
            Interface::Channel(Var { name, typ }) => {
                fields.push(format!("pub {name}_r: Channel<{typ}>"));
            }
            Interface::Writer(Var { name, typ }) => {
                fields.push(format!("pub {name}_w: Writer<{typ}>"));
            }
        }
    }
    format! {
        "
        pub struct {} {{
            {}
        }}
        ",
        st.name,
        itertools::join(fields, ","),
    }
}
fn generate_initializer(st: Struct) -> String {
    let mut fields = vec![];
    for field in st.fields {
        match field {
            Interface::Channel(Var { name, typ }) => {
                fields.push(format!("{name}_r"));
            }
            Interface::Writer(Var { name, typ }) => {
                fields.push(format!("{name}_w"));
            }
        }
    }
    format! {
        "
        {} {{
            {}
        }};
        ",
        st.name,
        itertools::join(fields, ","),
    }
}
fn generate_factory(chans: Vec<Chan>, pub_st: Struct, inn_st: Struct) -> String {
    let init_chans = itertools::join(
        chans.into_iter().map(|chan| {
            let w = format!("{}_w", chan.name);
            let r = format!("{}_r", chan.name);
            format!("let ({w}, {r}) = csplib::channel();")
        }),
        "",
    );

    let init_x = generate_initializer(pub_st.clone());
    let init_y = generate_initializer(inn_st.clone());
    format! {
        "
        impl {} {{
            pub fn new() -> ({}, {}) {{
                {init_chans}
                let x = {init_x}
                let y = {init_y}
                (x, y)
            }}
        }}
        ",
        pub_st.name,
        pub_st.name,
        inn_st.name,
    }
}

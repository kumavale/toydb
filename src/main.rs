use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
enum Type {
    Int(Option<i32>),
    Text(Option<String>),
}

#[derive(Clone, Debug)]
struct Table {
    name:     String,
    order:    Vec<String>,
    column:   HashMap<String, Type>,
    max_lens: HashMap<String, usize>,
    data:     Vec<HashMap<String, Type>>,
}

#[derive(Clone, Debug, PartialEq)]
enum LikeType {
    Underscore,   // '_'
    Percent,      // '%'
    Str(String),  // other characters
}

impl Table {
    fn new(name: &str, col: Vec<(&str, Type)>) -> Self {
        let mut order    = Vec::new();
        let mut column   = HashMap::new();
        let mut max_lens = HashMap::new();
        for c in col {
            order.push(c.0.to_owned());
            column.insert(c.0.to_owned(), c.1);
            max_lens.insert(c.0.to_owned(), c.0.len());
        }
        Table { name: name.to_owned(),
                order,
                column,
                max_lens,
                data: Vec::new(),
        }
    }

    fn insert(&mut self, data: Vec<(impl Into<String> + Copy, Type)>) {
        let mut hashmap = HashMap::new();
        for d in data {
            if self.column.get(&d.0.into()).is_some() {
                if hashmap.get(&d.0.into()).is_some() {
                    panic!("Duplicate column name \"{}\"", d.0.into());
                }
                let len = match &d.1 {
                    Type::Int(Some(i))  => i32_len(*i),
                    Type::Text(Some(t)) => t.len(),
                    _ => 4, // null
                };
                if self.max_lens[&d.0.into()] < len {
                    self.max_lens.insert(d.0.into(), len);
                }
                hashmap.insert(d.0.into(), d.1);
            } else {
                panic!("Unknown column \"{}\" in \"{}\"", d.0.into(), self.name);
            }
        }

        self.data.push(hashmap);
    }

    fn select(&self, cols: &[&str]) -> Table {
        let mut new_t = self.clone();
        new_t.column.clear();
        new_t.order.clear();
        for col in cols {
            if self.column.get(*col).is_some() {
                new_t.column.insert(col.to_owned().to_string(), self.column[*col].clone());
            }
        }
        new_t.order = cols.iter().map(|&c| c.to_string()).collect();
        new_t
    }

    fn less_than(&self, col: &str, num: i32) -> Table {
        let mut new_t = self.clone();
        new_t.data.clear();
        for d in &self.data {
            if let Type::Int(Some(n)) = d[col] {
                if n < num {
                    new_t.data.push(d.clone());
                }
            }
        }
        new_t
    }

    fn like(&self, col: &str, pattern: &str) -> Table {
        fn tokenize(pattern: &str) -> Vec<LikeType> {
            let mut tokens = vec![];
            let pattern_chars = pattern.chars().collect::<Vec<char>>();
            let mut idx = 0;
            while idx < pattern_chars.len() {
                match pattern_chars[idx] {
                    '_' => {
                        tokens.push(LikeType::Underscore);
                        idx += 1;
                    }
                    '%' => {
                        tokens.push(LikeType::Percent);
                        idx += 1;
                    }
                     _  => {
                         let mut s = pattern_chars[idx].to_string();
                         idx += 1;
                         while idx < pattern_chars.len() && pattern_chars[idx] != '%' && pattern_chars[idx] != '_' {
                             s.push(pattern_chars[idx]);
                             idx += 1;
                         }
                         tokens.push(LikeType::Str(s));
                     }
                }
            }
            tokens
        }
        let pattern = tokenize(pattern);
        let mut new_t = self.clone();
        new_t.data.clear();
        for d in &self.data {
            if let Type::Text(Some(s)) = &d[col] {
                if like(&pattern, &s) {
                    new_t.data.push(d.clone());
                }
            }
        }
        new_t
    }

    fn left_join(&self, other: &Table, key: &str) -> Table {
        let mut new_t = self.clone();

        if other.column.get(key).is_none() || new_t.column.get(key).is_none() {
            // error msg
            return new_t;
        }

        let mut other_cols = Vec::new();
        for o in &other.order {
            if new_t.column.get(o).is_none() {
                new_t.order.push(o.clone());
                new_t.column.insert(o.clone(), other.column[o].clone());
                new_t.max_lens.insert(o.clone(), other.max_lens[o]);
                other_cols.push(o.clone());
            }
        }

        for d in &mut new_t.data {
            for od in &other.data {
                if d.get(key) == od.get(key) {
                    for oc in &other_cols {
                        d.insert(oc.to_owned(), od.get(oc).unwrap().clone());
                    }
                }
            }
        }

        new_t
    }

    fn display(&self) {
        let line = || {
            print!(" +");
            for key in self.order.iter() {
                print!("-{:-<width$}-+", "-", width = self.max_lens[key]);
            }
            println!();
        };

        line();
        print!(" |");
        for col in self.order.iter() {
            print!(" {:^width$} |", col, width = self.max_lens[col]);
        }
        println!();
        line();

        for d in &self.data {
            for col in self.order.iter() {
                print!(" | ");
                if let Some(v) = d.get(col) {
                    match v {
                        Type::Int(Some(val))  => {
                            print!("{:>width$}", val, width = self.max_lens[col]);
                        },
                        Type::Text(Some(text)) => {
                            print!("{:<width$}", text, width = self.max_lens[col]);
                        },
                        _ => {
                            print!("{:>width$}", "NULL", width = self.max_lens[col]);
                        },
                    }
                } else {
                    print!("{:>width$}", "NULL", width = self.max_lens[col]);
                };
            }
            println!(" |");
        }

        line();
    }
}

fn i32_len(mut i: i32) -> usize {
    let mut len: usize = 0;
    if i < 0 {
        len += 1;
        i = -i;
    }

    while 0 < i {
        i /= 10;
        len += 1;
    }

    len
}

fn like(mut pattern: &[LikeType], mut target: &str) -> bool {
    loop {
        if target.is_empty() {
            return pattern.is_empty();
        }
        match pattern.first() {
            Some(LikeType::Underscore) => target = &target[1..],
            Some(LikeType::Percent) => {
                return pattern.len() == 1 || (1..target.len()).any(|i|like(&pattern[1..], &target[i..]));
            }
            Some(LikeType::Str(s)) => {
                if !target.starts_with(s) {
                    return false;
                }
                target = &target[s.len()..]
            }
            None => break
        }
        pattern = &pattern[1..];
    }
    target.is_empty()
}

fn main() {
    let mut table1 = Table::new( "table1",
        vec![ ("id",    Type::Int(None)),
              ("name",  Type::Text(None)),
              ("price", Type::Int(None)), ]);

    table1.insert(vec![("id",    Type::Int(Some(1))),
                       ("name",  Type::Text(Some("apple".to_owned()))),
                       ("price", Type::Int(Some(50)))]);
    table1.insert(vec![("id",    Type::Int(Some(2))),
                       ("name",  Type::Text(Some("banana".to_owned()))),
                       ("price", Type::Int(Some(100)))]);
    table1.insert(vec![("id",    Type::Int(Some(3))),
                       ("name",  Type::Text(Some("citrus".to_owned()))),
                       ("price", Type::Int(None))]);
    table1.insert(vec![("id",    Type::Int(Some(4))),
                       ("name",  Type::Text(Some("dorian".to_owned()))),
                       ("price", Type::Int(Some(256)))]);
    table1.insert(vec![("id",    Type::Int(Some(5))),
                       ("name",  Type::Text(Some("elderberries".to_owned()))),
                       ("price", Type::Int(Some(512)))]);
    table1.insert(vec![("id",    Type::Int(Some(6))),
                       ("name",  Type::Text(Some("figs".to_owned()))),
                       ("price", Type::Int(Some(1024)))]);
    table1.insert(vec![("id",    Type::Int(Some(7))),
                       ("name",  Type::Text(Some("grapefruit".to_owned()))),
                       ("price", Type::Int(Some(2048)))]);
    table1.insert(vec![("id",    Type::Int(Some(8))),
                       ("name",  Type::Text(Some("honeydew melon".to_owned()))),
                       ("price", Type::Int(Some(4096)))]);

    let mut table2 = Table::new("table2",
        vec![ ("id", Type::Int(None)),
              ("date", Type::Text(None)), ]);

    table2.insert(vec![("id", Type::Int(Some(1))),
                       ("date", Type::Text(Some("2019/12/20".to_owned())))]);
    table2.insert(vec![("id", Type::Int(Some(2))),
                       ("date", Type::Text(Some("2019/12/21".to_owned())))]);
    table2.insert(vec![("id", Type::Int(Some(3))),
                       ("date", Type::Text(Some("2019/12/22".to_owned())))]);
    table2.insert(vec![("id", Type::Int(Some(4))),
                       ("date", Type::Text(Some("2019/12/23".to_owned())))]);
    table2.insert(vec![("id", Type::Int(Some(8))),
                       ("date", Type::Text(Some("2019/12/27".to_owned())))]);
    table2.insert(vec![("id", Type::Int(Some(13))),
                       ("date", Type::Text(Some("2020/01/01".to_owned())))]);

    println!("\n====[ table1 ALL ]====");
    table1.display();

    println!("\n====[ table1 SELECT ]====");
    table1.select(&["name"]).display();
    table1.select(&["name", "price"]).display();

    println!("\n====[ table1 WHERE < ]====");
    table1.less_than("id", 10).display();
    table1.less_than("price", 250).display();

    println!("\n====[ table2 ALL ]====");
    table2.display();

    println!("\n====[ table1:table2 LEFT JOIN ]====");
    table1.left_join(&table2, "id").display();

    println!("\n====[ table1:table2 LEFT JOIN => SELECT ]====");
    table1.left_join(&table2, "id").select(&["name", "date"]).display();

    println!("\n====[ table1 WHERE LIKE ]====");
    table1.like("name", "apple").display();
    table1.like("name", "______").display();
    table1.like("name", "%s").display();
    table1.like("name", "%ri%").display();
}


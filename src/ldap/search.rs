pub struct SearchAttrs {
    attrs: Vec<String>,
}

impl Default for SearchAttrs {
    fn default() -> Self {
        SearchAttrs {
            attrs: vec![
                String::from("cn"),
                String::from("dn"),
                String::from("uid"),
                String::from("memberOf"),
                String::from("krbPrincipalName"),
                String::from("mail"),
                String::from("mobile"),
                String::from("ibutton"),
                String::from("drinkBalance"),
            ],
        }
    }
}

impl SearchAttrs {
    pub fn new(attrs: Vec<&str>) -> Self {
        SearchAttrs {
            attrs: attrs
                .iter()
                .map(|attr| attr.to_owned().to_owned())
                .collect(),
        }
    }

    pub fn finalize(self) -> Vec<String> {
        self.attrs.iter().map(|attr| attr.to_owned()).collect()
    }
}

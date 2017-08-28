extern crate skeptic;

use std::borrow::Cow;
use std::env;
use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::path::Path;

type Map<K, V> = BTreeMap<K, V>;

mod channel;
mod connection;
mod server;

#[derive(Default, Clone)]
struct Property<'a> {
	name: Cow<'a, str>,
	type_s: Cow<'a, str>,
	/// If the property should be wrapped into a result.
	result: bool,
	documentation: Cow<'a, str>,
	initialise: bool,
	/// The code that creates the content of this property.
	initialisation: Option<Cow<'a, str>>,
	/// The code that updates the content of this property.
	update: Option<Cow<'a, str>>,
	/// If an update method should be generated for this property.
	should_update: bool,
	/// Use a fixed function
	method_name: Option<Cow<'a, str>>,
	/// The name that is used to initialise this value: enum_name::value_name
	enum_name: Cow<'a, str>,
	value_name: Option<Cow<'a, str>>,
	/// Map type_s → used function
	functions: Map<Cow<'a, str>, Cow<'a, str>>,
	/// Types that are transmutable, the standard type that is taken is int.
	transmutable: Vec<Cow<'a, str>>,
	/// Arguments passed to the function
	default_args: Cow<'a, str>,
	/// Arguments passed to the function when updating the property.
	default_args_update: Cow<'a, str>,
	/// If an api getter should be created for this property.
	api_getter: bool,
	/// If the getter method should be public.
	public: bool,
}

impl<'a> Property<'a> {
	fn create_attribute(&self) -> String {
		let mut s = String::new();
		if !self.documentation.is_empty() {
			s.push_str(self.documentation.lines()
				.map(|l| format!("/// {}\n", l)).collect::<String>().as_str());
		}
		if self.result {
			s.push_str(format!("{}: Result<{}, ::Error>,\n", self.name, self.type_s).as_str());
		} else {
			s.push_str(format!("{}: {},\n", self.name, self.type_s).as_str());
		}
		s
	}

	fn create_return_type(&self, is_ref_type: bool) -> String {
		// Build the result type
		let mut result_type = String::new();
		if self.result {
			result_type .push_str("Result<")
		}
		if is_ref_type {
			result_type.push('&');
		}
		if self.type_s == "String" {
			result_type.push_str("str");
		} else {
			result_type.push_str(self.type_s.as_ref());
		}
		if self.result {
			result_type.push_str(", ");
			if is_ref_type {
				//result_type.push('&');
			}
			result_type.push_str("::Error>");
		}
		result_type
	}

	fn create_getter(&self) -> String {
		let is_ref_type = ["String", "Permissions"].contains(&self.type_s.as_ref())
			|| self.type_s.starts_with("Option") || self.type_s.starts_with("Map<")
			|| self.type_s.starts_with("Vec<");;

		let mut s = String::new();
		// Create the getter
		if self.public {
			s.push_str("pub ");
		}
		s.push_str(format!("fn get_{}(&self) -> {} {{\n", self.name, self.create_return_type(is_ref_type)).as_str());
		s.push_str(indent("", 1).as_str());
		let mut body = String::new();
		if !self.result && is_ref_type {
			body.push('&');
		}
		body.push_str(format!("self.{}", self.name).as_str());
		if self.result && is_ref_type {
			body.push_str(".as_ref()");
			if self.type_s == "String" {
				body.push_str(".map(|v| v.as_str())");
			}
			body.push_str(".map_err(|e| *e)");
		}
		body.push('\n');
		s.push_str(indent(body, 1).as_str());
		s.push_str("}\n");
		s
	}

	fn create_api_getter(&self) -> String {
		if !self.api_getter {
			return String::new();
		}
		let is_ref_type = ["String", "Permissions"].contains(&self.type_s.as_ref())
			|| self.type_s.starts_with("Option") || self.type_s.starts_with("Map<")
			|| self.type_s.starts_with("Vec<");;

		let mut s = String::new();
		// Create the getter
		s.push_str(format!("pub fn get_{}(&self) -> {} {{\n", self.name, self.create_return_type(is_ref_type)).as_str());
		s.push_str(indent(format!("\
			match self.data {{\n\
				\tOk(data) => data.get_{}(),\n\
				\tErr(_) => Err(Error::Ok),\n\
			}}\n", self.name), 1).as_str());
		s.push_str("}\n");
		s
	}

	fn create_update(&self) -> String {
		let mut s = String::new();
		let initialisation = self.intern_create_initialisation(self.default_args_update.as_ref(), true);
		if !initialisation.is_empty() {
			// Create the update function
			s.push_str(format!("fn update_{}(&mut self) {{\n", self.name).as_str());
			s.push_str(indent(format!("self.{} = {};\n", self.name, initialisation), 1).as_str());
			s.push_str("}\n");
		}
		s
	}

	fn create_initialisation(&self) -> String {
		if self.result {
			String::from("Err(::Error::Ok)")
		} else {
			self.intern_create_initialisation(self.default_args.as_ref(), false)
		}
	}

	fn intern_create_initialisation(&self, default_args: &str, update: bool) -> String {
		if !self.initialise || (update && !self.should_update) {
			return String::new();
		} else if update && self.update.is_some() {
			return self.update.as_ref().unwrap().clone().into_owned();
		} else if self.initialisation.is_some() {
			return self.initialisation.as_ref().unwrap().clone().into_owned();
		}
		let value_name = self.value_name.as_ref().map(|s| s.clone()).unwrap_or(to_pascal_case(self.name.as_ref()).into());
		let mut s = String::new();
		// Ignore unknown types
		if let Some(function) = self.method_name.as_ref() {
			// Special defined function
			s.push_str(format!("{}({}{}::{})", function, default_args,
				self.enum_name, value_name).as_str());
		} else if let Some(function) = self.functions.get(self.type_s.as_ref()) {
			// From function list
			s.push_str(format!("{}({}{}::{})", function, default_args,
				self.enum_name, value_name).as_str());
		} else if self.transmutable.contains(&self.type_s) {
			// Try to get an int
			for t in &["i32", "u64"] {
				if let Some(function) = self.functions.get(*t) {
					s.push_str(format!("{}({}{}::{}).map(|v| unsafe {{ transmute(v) }})", function, default_args,
						self.enum_name, value_name).as_str());
					break;
				}
			}
		} else {
			match self.type_s.as_ref() {
				"Duration" => {
					// Try to get an u64
					let function: &str = if let Some(f) = self.functions.get("u64") {
						f
					} else if let Some(f) = self.functions.get("i32") {
						f
					} else {
						"get_property_as_int"
					};
					s.push_str(format!("{}({}{}::{}).map(|d| Duration::seconds(d as i64))",
						function, default_args, self.enum_name, value_name).as_str())
				},
				"DateTime<Utc>" => {
					// Try to get an u64
					let function: &str = if let Some(f) = self.functions.get("u64") {
						f
					} else if let Some(f) = self.functions.get("i32") {
						f
					} else {
						"get_property_as_int"
					};
					s.push_str(format!("{}({}{}::{}).map(|d| DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(d as i64, 0), chrono::Utc))",
						function, default_args, self.enum_name, value_name).as_str())
				},
				"bool" => {
					for t in &["i32", "u64"] {
						if let Some(function) = self.functions.get(*t) {
							s.push_str(format!("{}({}{}::{}).map(|v| v != 0)", function,
								default_args, self.enum_name, value_name).as_str());
							break;
						}
					}
				}
				_ => {}
			}
		}
		s
	}
}

#[derive(Default, Clone)]
struct PropertyBuilder<'a> {
	name: Cow<'a, str>,
	type_s: Cow<'a, str>,
	result: bool,
	documentation: Cow<'a, str>,
	initialise: bool,
	initialisation: Option<Cow<'a, str>>,
	update: Option<Cow<'a, str>>,
	should_update: bool,
	method_name: Option<Cow<'a, str>>,
	enum_name: Cow<'a, str>,
	value_name: Option<Cow<'a, str>>,
	functions: Map<Cow<'a, str>, Cow<'a, str>>,
	transmutable: Vec<Cow<'a, str>>,
	default_args: Cow<'a, str>,
	default_args_update: Cow<'a, str>,
	api_getter: bool,
	public: bool,
}

#[allow(dead_code)]
impl<'a> PropertyBuilder<'a> {
	fn new() -> PropertyBuilder<'a> {
		let mut result = Self::default();
		result.initialise = true;
		result.result = true;
		result.should_update = true;
		result.api_getter = true;
		result.public = true;
		result
	}

	fn name<S: Into<Cow<'a, str>>>(&self, name: S) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.name = name.into();
		res
	}

	fn type_s<S: Into<Cow<'a, str>>>(&self, type_s: S) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.type_s = type_s.into();
		res
	}

	fn result(&self, result: bool) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.result = result;
		res
	}

	fn documentation<S: Into<Cow<'a, str>>>(&self, documentation: S) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.documentation = documentation.into();
		res
	}

	fn initialise(&self, initialise: bool) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.initialise = initialise;
		res
	}

	fn initialisation<S: Into<Cow<'a, str>>>(&self, initialisation: S) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.initialisation = Some(initialisation.into());
		res
	}

	fn update<S: Into<Cow<'a, str>>>(&self, update: S) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.update = Some(update.into());
		res
	}

	fn should_update(&self, should_update: bool) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.should_update = should_update.into();
		res
	}

	fn method_name<S: Into<Cow<'a, str>>>(&self, method_name: S) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.method_name = Some(method_name.into());
		res
	}

	fn enum_name<S: Into<Cow<'a, str>>>(&self, enum_name: S) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.enum_name = enum_name.into();
		res
	}

	fn value_name<S: Into<Cow<'a, str>>>(&self, value_name: S) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.value_name = Some(value_name.into());
		res
	}

	fn functions<S1: Into<Cow<'a, str>>, S2: Into<Cow<'a, str>>>(&self, functions: Map<S1, S2>) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.functions = functions.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
		res
	}

	fn transmutable<S: Into<Cow<'a, str>>>(&self, transmutable: Vec<S>) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.transmutable = transmutable.into_iter().map(|s| s.into()).collect();
		res
	}

	fn default_args<S: Into<Cow<'a, str>>>(&self, default_args: S) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.default_args = default_args.into();
		res
	}

	fn default_args_update<S: Into<Cow<'a, str>>>(&self, default_args_update: S) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.default_args_update = default_args_update.into();
		res
	}

	fn api_getter(&self, api_getter: bool) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.api_getter = api_getter;
		res
	}

	fn public(&self, public: bool) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.public = public;
		res
	}

	fn finalize(self) -> Property<'a> {
		Property {
			name: self.name,
			type_s: self.type_s,
			result: self.result,
			documentation: self.documentation,
			initialise: self.initialise,
			initialisation: self.initialisation,
			update: self.update,
			should_update: self.should_update,
			method_name: self.method_name,
			enum_name: self.enum_name,
			value_name: self.value_name,
			functions: self.functions.clone(),
			transmutable: self.transmutable.clone(),
			default_args: self.default_args,
			default_args_update: self.default_args_update,
			api_getter: self.api_getter,
			public: self.public,
		}
	}
}

struct Struct<'a> {
	/// The name of this struct
	name: Cow<'a, str>,
	/// The name of this struct when exposed by the api
	api_name: Cow<'a, str>,
	/// The documentation of this struct
	documentation: Cow<'a, str>,
	/// Members that will be generated for this struct
	properties: Vec<Property<'a>>,
	/// Code that will be put into the struct part
	extra_attributes: Cow<'a, str>,
	/// Code that will be inserted into the constructor (::new method)
	extra_initialisation: Cow<'a, str>,
	/// Code that will be inserted into the creation of the struct
	extra_creation: Cow<'a, str>,
	/// Arguments that are taken by the constructor
	constructor_args: Cow<'a, str>,
	/// If the resulting struct is public
	public: bool,
}

#[derive(Default, Clone)]
struct StructBuilder<'a> {
	name: Cow<'a, str>,
	api_name: Cow<'a, str>,
	documentation: Cow<'a, str>,
	properties: Vec<Property<'a>>,
	extra_attributes: Cow<'a, str>,
	extra_initialisation: Cow<'a, str>,
	extra_creation: Cow<'a, str>,
	constructor_args: Cow<'a, str>,
	public: bool,
}

impl<'a> StructBuilder<'a> {
	fn new() -> StructBuilder<'a> {
		let mut result = Self::default();
		result.public = true;
		result
	}

	fn name<S: Into<Cow<'a, str>>>(&mut self, name: S) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.name = name.into();
		res
	}

	fn api_name<S: Into<Cow<'a, str>>>(&mut self, api_name: S) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.api_name = api_name.into();
		res
	}

	fn documentation<S: Into<Cow<'a, str>>>(&mut self, documentation: S) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.documentation = documentation.into();
		res
	}

	fn properties(&mut self, properties: Vec<Property<'a>>) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.properties = properties;
		res
	}

	fn extra_attributes<S: Into<Cow<'a, str>>>(&mut self, extra_attributes: S) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.extra_attributes = extra_attributes.into();
		res
	}

	fn extra_initialisation<S: Into<Cow<'a, str>>>(&mut self, extra_initialisation: S) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.extra_initialisation = extra_initialisation.into();
		res
	}

	fn extra_creation<S: Into<Cow<'a, str>>>(&mut self, extra_creation: S) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.extra_creation = extra_creation.into();
		res
	}

	fn constructor_args<S: Into<Cow<'a, str>>>(&mut self, constructor_args: S) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.constructor_args = constructor_args.into();
		res
	}

	fn public(&mut self, public: bool) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.public = public;
		res
	}

	fn finalize(self) -> Struct<'a> {
		Struct {
			name: self.name,
			api_name: self.api_name,
			documentation: self.documentation,
			// Move the contents of the properties
			properties: self.properties.clone(),
			extra_attributes: self.extra_attributes,
			extra_initialisation: self.extra_initialisation,
			extra_creation: self.extra_creation,
			constructor_args: self.constructor_args,
			public: self.public,
		}
	}
}

impl<'a> Struct<'a> {
	fn create_struct(&self) -> String {
		let mut s = String::new();
		for prop in &self.properties {
			s.push_str(prop.create_attribute().as_str());
		}
		let mut result = String::new();
		if !self.documentation.is_empty() {
			result.push_str(format!("/// {}\n", self.documentation).as_str());
		}
		result.push_str("#[derive(Clone)]\n");
		if self.public {
			result.push_str("pub ");
		}
		result.push_str(format!("struct {} {{\n{}", self.name, indent(s, 1)).as_str());
		if !self.extra_attributes.is_empty() {
			result.push_str(format!("\n{}", indent(self.extra_attributes.as_ref(), 1)).as_str());
		}
		result.push_str("}\n\n");
		result
	}

	fn create_impl(&self) -> String {
		let mut s = String::new();
		for prop in &self.properties {
			s.push_str(prop.create_getter().as_str());
		}
		let mut result = String::new();
		write!(result, "impl {} {{\n{}}}\n\n", self.name, indent(s, 1)).unwrap();
		result
	}

	fn create_api_impl(&self) -> String {
		let mut s = String::new();
		for prop in &self.properties {
			s.push_str(prop.create_api_getter().as_str());
		}
		let mut result = String::new();
		write!(result, "impl<'a> {}<'a> {{\n{}}}\n\n", self.api_name, indent(s, 1)).unwrap();
		result
	}

	fn create_update(&self) -> String {
		// The content that holds all update methods
		let mut s = String::new();
		// The update() method
		let mut updates = String::new();

		for prop in &self.properties {
			let update = prop.create_update();
			if !update.is_empty() {
				s.push_str(update.as_str());
				updates.push_str(format!("self.update_{}();\n", prop.name).as_str());
			}
		}
		// Add an update method for everything
		s.push_str("\nfn update(&mut self) {\n");
		s.push_str(indent(updates, 1).as_str());
		s.push_str("}\n");

		// Add update_from
		let mut updates = String::new();
		for prop in &self.properties {
			if prop.result {
				updates.push_str(format!("if self.{}.is_err() {{\n", prop.name).as_str());
				updates.push_str(indent(format!("self.{0} = other.{0}.clone();", prop.name), 1).as_str());
				updates.push_str("}\n");
			}
		}
		s.push_str("fn update_from(&mut self, other: &Self) {\n");
		s.push_str(indent(updates, 1).as_str());
		s.push_str("}\n");

		let mut result = String::new();
		write!(result, "impl {} {{\n{}}}\n\n", self.name, indent(s, 1)).unwrap();
		result
	}

	/// struct_name: Name of the struct
	/// properties_name: Name of the properties enum
	/// args: Base args (id) to get properties
	fn create_constructor(&self) -> String {
		let mut inits = String::new();
		// Initialisation
		if !self.extra_initialisation.is_empty() {
			inits.push_str(self.extra_initialisation.as_ref());
			inits.push('\n');
		}
		// Creation
		let mut creats = String::new();
		for prop in &self.properties {
			let p = prop.create_initialisation();
			let initialisation = if p.is_empty() {
				prop.name.clone().into_owned()
			} else {
				p
			};
			creats.push_str(format!("{}: {},\n", prop.name, initialisation).as_str());
		}
		if !self.extra_creation.is_empty() {
			creats.push('\n');
			creats.push_str(self.extra_creation.as_ref());
		}

		let mut result = String::new();
		write!(result, "impl {0} {{
	fn new({1}) -> {0} {{
{2}
		{0} {{
{3}
		}}
	}}\n}}\n\n", self.name, self.constructor_args, indent(inits, 2), indent(creats, 3)).unwrap();
		result
	}
}

/// Build parts of lib.rs as most of the structs are very repetitive
fn main() {
	let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
	println!("cargo:rerun-if-changed={}/build/build.rs", manifest_dir);
	println!("cargo:rerun-if-changed={}/README.md", manifest_dir);

	let out_dir = env::var("OUT_DIR").unwrap();
	let dest_path = Path::new(&out_dir).join("structs.rs");
	let mut f = File::create(&dest_path).unwrap();

	channel::create(&mut f);
	connection::create(&mut f);
	server::create(&mut f);

	// Create tests for README.md
	skeptic::generate_doc_tests(&["README.md"]);
}

fn to_pascal_case<S: AsRef<str>>(text: S) -> String {
	let sref = text.as_ref();
	let mut s = String::with_capacity(sref.len());
	let mut uppercase = true;
	for c in sref.chars() {
		if c == '_' {
			uppercase = true;
		} else {
			if uppercase {
				s.push(c.to_uppercase().next().unwrap());
				uppercase = false;
			} else {
				s.push(c);
			}
		}
	}
	s
}

/// Indent a string by a given count using spaces.
fn indent<S: AsRef<str>>(s: S, count: usize) -> String {
	let sref = s.as_ref();
	let line_count = sref.lines().count();
	let mut result = String::with_capacity(sref.len() + line_count * count * 4);
	for l in sref.lines() {
		if !l.is_empty() {
			result.push_str(std::iter::repeat("\t").take(count).collect::<String>().as_str());
		}
		result.push_str(l);
		result.push('\n');
	}
	result
}

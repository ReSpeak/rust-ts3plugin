// Limit for error_chain
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
extern crate serde;
#[macro_use]
extern crate tera;

use std::borrow::Cow;
use std::env;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use serde::ser::SerializeStruct;
use tera::Tera;

type Map<K, V> = BTreeMap<K, V>;

#[allow(unused_doc_comment)]
mod errors {
	// Create the Error, ErrorKind, ResultExt, and Result types
	error_chain! {
		foreign_links {
			Io(::std::io::Error);
			EnvVar(::std::env::VarError);
			Tera(::tera::Error);
		}
	}
}
use errors::*;

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
	/// If this property needs to be requested.
	requested: bool,
}

impl<'a> Property<'a> {
	fn is_ref_type(&self) -> bool {
		["String", "Permissions"].contains(&self.type_s.as_ref())
			|| self.type_s.starts_with("Option") || self.type_s.starts_with("Map<")
			|| self.type_s.starts_with("Vec<")
	}

	fn create_return_type(&self) -> String {
		// Build the result type
		let is_ref_type = self.is_ref_type();
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
			result_type.push_str("::Error>");
		}
		result_type
	}

	fn create_ref_type(&self) -> String {
		// Build the result type
		let is_ref_type = self.is_ref_type();
		let mut result_type = String::new();
		if is_ref_type {
			result_type.push_str("&'a ");
		}
		if self.type_s == "String" {
			result_type.push_str("str");
		} else {
			result_type.push_str(self.type_s.as_ref());
		}
		result_type
	}

	fn create_getter_body(&self) -> String {
		let is_ref_type = self.is_ref_type();
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
		body
	}

	fn create_constructor_body(&self) -> String {
		let p = self.create_initialisation();
		if p.is_empty() {
			self.name.clone().into_owned()
		} else {
			p
		}
	}

	fn create_update_body(&self) -> String {
		self.intern_create_initialisation(self.default_args_update.as_ref(), true)
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

impl<'a> serde::Serialize for Property<'a> {
	fn serialize<S: serde::Serializer>(&self, serializer: S)
		-> std::result::Result<S::Ok, S::Error> {
		let mut s = serializer.serialize_struct("Property", 22)?;

		// Attributes
		s.serialize_field("name", &self.name)?;
		s.serialize_field("type_s", &self.type_s)?;
		s.serialize_field("result", &self.result)?;
		let documentation = self.documentation.lines()
			.map(|l| format!("/// {}\n", l)).collect::<String>();
		s.serialize_field("documentation", &documentation)?;
		s.serialize_field("initialise", &self.initialise)?;
		s.serialize_field("initialisation", &self.initialisation)?;
		s.serialize_field("update", &self.update)?;
		s.serialize_field("should_update", &self.should_update)?;
		s.serialize_field("method_name", &self.method_name)?;
		s.serialize_field("enum_name", &self.enum_name)?;
		s.serialize_field("value_name", &self.value_name)?;
		s.serialize_field("functions", &self.functions)?;
		s.serialize_field("transmutable", &self.transmutable)?;
		s.serialize_field("default_args", &self.default_args)?;
		s.serialize_field("default_args_update", &self.default_args_update)?;
		s.serialize_field("api_getter", &self.api_getter)?;
		s.serialize_field("public", &self.public)?;
		s.serialize_field("requested", &self.requested)?;

		// Extra attributes
		s.serialize_field("return_type", &self.create_return_type())?;
		s.serialize_field("getter_body", &self.create_getter_body())?;
		s.serialize_field("constructor_body", &self.create_constructor_body())?;
		s.serialize_field("update_body", &self.create_update_body())?;

		s.end()
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
	requested: bool,
}

#[allow(dead_code)]
impl<'a> PropertyBuilder<'a> {
	fn new() -> PropertyBuilder<'a> {
		let mut result = Self::default();
		result.initialise = true;
		result.result = true;
		result.should_update = true;
		result.api_getter = true;
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

	fn requested(&self, requested: bool) -> PropertyBuilder<'a> {
		let mut res = self.clone();
		res.requested = requested;
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
			requested: self.requested,
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
	/// Code that will be inserted into the Implementation of the struct
	extra_implementation: Cow<'a, str>,
	/// Code that will be inserted into the PropertyType enum
	extra_property_type: Cow<'a, str>,
	/// Code that will be inserted into the PropertyList enum
	/// (type, ref type, enum name)
	extra_property_list: Vec<(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>)>,
	/// Code that will be inserted into the properties() function
	extra_properties: Cow<'a, str>,
	/// Arguments that are taken by the constructor
	constructor_args: Cow<'a, str>,
	/// If the resulting struct is public
	public: bool,
	// What should be done for this struct
	do_struct: bool,
	do_impl: bool,
	do_api_impl: bool,
	do_update: bool,
	do_constructor: bool,
	do_properties: bool,
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
	extra_implementation: Cow<'a, str>,
	extra_property_type: Cow<'a, str>,
	extra_property_list: Vec<(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>)>,
	extra_properties: Cow<'a, str>,
	constructor_args: Cow<'a, str>,
	public: bool,
	do_struct: bool,
	do_impl: bool,
	do_api_impl: bool,
	do_update: bool,
	do_constructor: bool,
	do_properties: bool,
}

#[allow(dead_code)]
impl<'a> StructBuilder<'a> {
	fn new() -> StructBuilder<'a> {
		let mut result = Self::default();
		result.do_struct = true;
		result.do_impl = true;
		result.do_constructor = true;
		result.do_update = true;
		result.do_api_impl = false;
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

	fn extra_property_type<S: Into<Cow<'a, str>>>(&mut self, extra_property_type: S) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.extra_property_type = extra_property_type.into();
		res
	}

	fn extra_property_list(&mut self, extra_property_list: Vec<(Cow<'a, str>, Cow<'a, str>, Cow<'a, str>)>) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.extra_property_list = extra_property_list;
		res
	}

	fn extra_properties<S: Into<Cow<'a, str>>>(&mut self, extra_properties: S) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.extra_properties = extra_properties.into();
		res
	}

	fn extra_creation<S: Into<Cow<'a, str>>>(&mut self, extra_creation: S) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.extra_creation = extra_creation.into();
		res
	}

	fn extra_implementation<S: Into<Cow<'a, str>>>(&mut self, extra_implementation: S) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.extra_implementation = extra_implementation.into();
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

	fn do_struct(&mut self, do_struct: bool) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.do_struct = do_struct;
		res
	}

	fn do_impl(&mut self, do_impl: bool) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.do_impl = do_impl;
		res
	}

	fn do_api_impl(&mut self, do_api_impl: bool) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.do_api_impl = do_api_impl;
		res
	}

	fn do_update(&mut self, do_update: bool) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.do_update = do_update;
		res
	}

	fn do_constructor(&mut self, do_constructor: bool) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.do_constructor = do_constructor;
		res
	}

	fn do_properties(&mut self, do_properties: bool) -> StructBuilder<'a> {
		let mut res = self.clone();
		res.do_properties = do_properties;
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
			extra_implementation: self.extra_implementation,
			extra_property_type: self.extra_property_type,
			extra_property_list: self.extra_property_list,
			extra_properties: self.extra_properties,
			constructor_args: self.constructor_args,
			public: self.public,
			do_struct: self.do_struct,
			do_impl: self.do_impl,
			do_api_impl: self.do_api_impl,
			do_update: self.do_update,
			do_constructor: self.do_constructor,
			do_properties: self.do_properties,
		}
	}
}

impl<'a> Struct<'a> {
	fn create_struct(&self, f: &mut Write, tera: &Tera, all_structs: &[&Struct<'static>]) -> Result<()> {
		let mut context = tera::Context::new();
		context.add("s", &self);
		context.add("all_structs", &all_structs);

		// Assemble properties
		let mut properties = Vec::<&Property<'static>>::new();
		for s in all_structs.iter().filter(|s| s.api_name == self.api_name) {
			for p in &s.properties {
				if properties.iter().all(|p2| p.name != p2.name) {
					properties.push(p);
				}
			}
		}
		context.add("properties", &properties);

		// Assemble property_types
		let mut property_types = Vec::<(_, _)>::new();
		for s in all_structs.iter().filter(|s| s.api_name == self.api_name) {
			for p in &s.properties {
				let t = p.type_s.to_string();
				if p.result && p.api_getter && property_types.iter().all(|p| p.0 != t) {
					property_types.push((t, p.create_ref_type()));
				}
			}
			for &(ref t, ref r, _) in &s.extra_property_list {
				let t = t.as_ref();
				let r = r.as_ref();
				if property_types.iter().all(|p| p.0 != t) {
					property_types.push((r.to_string(), t.to_string()));
				}
			}
		}
		context.add("property_types", &property_types);

		let s = tera.render("struct.rs.tera", &context)?;
		f.write_all(s.as_bytes())?;
		Ok(())
	}
}

impl<'a> serde::Serialize for Struct<'a> {
	fn serialize<S: serde::Serializer>(&self, serializer: S)
		-> std::result::Result<S::Ok, S::Error> {
		let mut s = serializer.serialize_struct("Struct", 19)?;

		// Attributes
		s.serialize_field("name", &self.name)?;
		s.serialize_field("api_name", &self.api_name)?;
		let documentation = self.documentation.lines()
			.map(|l| format!("/// {}\n", l)).collect::<String>();
		s.serialize_field("documentation", &documentation)?;
		s.serialize_field("properties", &self.properties)?;
		s.serialize_field("extra_attributes", &self.extra_attributes)?;
		s.serialize_field("extra_initialisation", &self.extra_initialisation)?;
		s.serialize_field("extra_creation", &self.extra_creation)?;
		s.serialize_field("extra_implementation", &self.extra_implementation)?;
		s.serialize_field("extra_property_type", &self.extra_property_type)?;
		s.serialize_field("extra_property_list", &self.extra_property_list)?;
		s.serialize_field("extra_properties", &self.extra_properties)?;
		s.serialize_field("constructor_args", &self.constructor_args)?;
		s.serialize_field("public", &self.public)?;
		s.serialize_field("do_struct", &self.do_struct)?;
		s.serialize_field("do_impl", &self.do_impl)?;
		s.serialize_field("do_api_impl", &self.do_api_impl)?;
		s.serialize_field("do_update", &self.do_update)?;
		s.serialize_field("do_constructor", &self.do_constructor)?;
		s.serialize_field("do_properties", &self.do_properties)?;

		s.end()
	}
}

/// Build parts of lib.rs as most of the structs are very repetitive
quick_main!(|| -> Result<()> {
	let out_dir = env::var("OUT_DIR")?;
	for f in &["build.rs", "channel.rs", "connection.rs", "server.rs",
		"struct.rs.tera", "macros.tera"] {
		println!("cargo:rerun-if-changed=build/{}", f);
	}

	let mut tera = compile_templates!("build/*.tera");
	tera.register_filter("indent", |value, args| {
		if let Some(&tera::Value::Number(ref n)) = args.get("depth") {
			if let tera::Value::String(s) = value {
				if let Some(n) = n.as_u64() {
					Ok(tera::Value::String(indent(s, n as usize)))
				} else {
					Err("the indent depth must be a number".into())
				}
			} else {
				Err("indent expects a string to filter".into())
			}
		} else {
			Err("Expected argument 'depth' for indent".into())
		}
	});
	tera.register_filter("title", |value, _| {
		if let tera::Value::String(s) = value {
			Ok(tera::Value::String(to_pascal_case(s)))
		} else {
			Err("title expects a string to filter".into())
		}
	});
	tera.register_filter("simplify", |value, _| {
		if let tera::Value::String(s) = value {
			Ok(tera::Value::String(s.replace(&['<', '>', ',', ' '] as &[char], "")))
		} else {
			Err("title expects a string to filter".into())
		}
	});

	let path = Path::new(&out_dir);

	let channel_f = File::create(&path.join("channel.rs"))?;
	let connection_f = File::create(&path.join("connection.rs"))?;
	let server_f = File::create(&path.join("server.rs"))?;

	let mut files = vec![channel_f, connection_f, server_f];
	let structs = vec![
		channel::create(),
		connection::create(),
		server::create(),
	];

	let mut all_structs = Vec::new();
	for s in &structs {
		all_structs.extend(s);
	}

	for (i, ss) in structs.iter().enumerate() {
		let f = &mut files[i];
		for s in ss {
			s.create_struct(f, &tera, all_structs.as_slice())?;
		}
	}

	Ok(())
});

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

/// Indent a string by a given count using tabs.
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

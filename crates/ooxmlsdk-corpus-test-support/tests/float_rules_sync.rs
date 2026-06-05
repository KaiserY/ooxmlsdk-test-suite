use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[test]
#[ignore = "requires sibling ooxmlsdk checkout to compare against ooxmlsdk/data"]
fn schema_float_rules_match_ooxmlsdk_data() {
    let expected: SchemaFloatRules =
        serde_json::from_str(include_str!("../data/schema-float-rules.json"))
            .expect("parse checked-in schema-float-rules.json");
    let generated = generate_float_rules_from_ooxmlsdk_data();

    if expected != generated {
        let generated_json =
            serde_json::to_string_pretty(&generated).expect("serialize generated float rules");
        let workspace_dir = workspace_dir();
        let out_file = workspace_dir.join("target/schema-float-rules.generated.json");
        fs::create_dir_all(
            out_file
                .parent()
                .expect("generated output should have parent"),
        )
        .expect("create target dir");
        fs::write(&out_file, format!("{generated_json}\n")).expect("write generated float rules");

        panic!(
            "checked-in schema-float-rules.json is out of sync with ooxmlsdk/data; generated rules written to {}",
            out_file.display()
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SchemaFloatRules {
    attrs: Vec<SchemaFloatAttrRule>,
    texts: Vec<SchemaFloatTextRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct SchemaFloatAttrRule {
    element: String,
    attr: String,
    kind: SchemaFloatKind,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct SchemaFloatTextRule {
    element: String,
    kind: SchemaFloatKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SchemaFloatKind {
    Single,
    Double,
}

fn generate_float_rules_from_ooxmlsdk_data() -> SchemaFloatRules {
    let ooxmlsdk_dir = workspace_dir()
        .parent()
        .expect("workspace should have a parent directory")
        .join("ooxmlsdk");
    let data_dir = ooxmlsdk_dir.join("data");
    let namespace_file = data_dir.join("namespaces.json");
    let schemas_dir = data_dir.join("schemas");

    let namespaces = read_namespaces(&namespace_file);
    let mut attrs = BTreeSet::new();
    let mut texts = BTreeSet::new();

    for entry in fs::read_dir(&schemas_dir).unwrap_or_else(|err| {
        panic!(
            "read ooxmlsdk data schema dir {}: {err}",
            schemas_dir.display()
        )
    }) {
        let entry = entry.expect("read generated schema entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        collect_schema_float_rules_from_data(&path, &namespaces, &mut attrs, &mut texts);
    }

    SchemaFloatRules {
        attrs: attrs.into_iter().collect(),
        texts: texts.into_iter().collect(),
    }
}

fn workspace_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize workspace dir")
}

fn read_namespaces(path: &Path) -> BTreeMap<String, String> {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("read namespaces.json {}: {err}", path.display()));
    let values: Vec<serde_json::Value> = serde_json::from_str(&raw).expect("parse namespaces.json");
    values
        .into_iter()
        .filter_map(|value| {
            let prefix = value.get("Prefix")?.as_str()?.to_string();
            let uri = value.get("Uri")?.as_str()?.to_string();
            Some((prefix, uri))
        })
        .collect()
}

fn collect_schema_float_rules_from_data(
    path: &Path,
    namespaces: &BTreeMap<String, String>,
    attrs: &mut BTreeSet<SchemaFloatAttrRule>,
    texts: &mut BTreeSet<SchemaFloatTextRule>,
) {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("read data schema {}: {err}", path.display()));
    let schema: serde_json::Value = serde_json::from_str(&raw).expect("parse data schema");
    let Some(types) = schema.get("Types").and_then(serde_json::Value::as_array) else {
        return;
    };
    let type_by_name = types
        .iter()
        .filter_map(|ty| Some((ty.get("ClassName")?.as_str()?, ty)))
        .collect::<BTreeMap<_, _>>();

    for ty in types {
        let Some(element_qname) = ty.get("Name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let element_qname = element_qname.rsplit('/').next().unwrap_or(element_qname);
        if element_qname.is_empty() {
            continue;
        }
        let element_name = expand_generated_qname(element_qname, namespaces, false);
        collect_type_float_attrs(ty, &type_by_name, namespaces, &element_name, attrs);
        collect_type_float_text(ty, &type_by_name, &element_name, texts);
    }
}

fn collect_type_float_attrs(
    ty: &serde_json::Value,
    type_by_name: &BTreeMap<&str, &serde_json::Value>,
    namespaces: &BTreeMap<String, String>,
    element_name: &str,
    attrs: &mut BTreeSet<SchemaFloatAttrRule>,
) {
    if let Some(base_name) = ty.get("BaseClass").and_then(serde_json::Value::as_str)
        && let Some(base_ty) = type_by_name.get(base_name)
    {
        collect_type_float_attrs(base_ty, type_by_name, namespaces, element_name, attrs);
    }

    let Some(type_attrs) = ty.get("Attributes").and_then(serde_json::Value::as_array) else {
        return;
    };
    for attr in type_attrs {
        let Some(type_ref) = attr.get("Type").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(kind) = float_kind(type_ref) else {
            continue;
        };
        let Some(attr_qname) = attr.get("QName").and_then(serde_json::Value::as_str) else {
            continue;
        };
        attrs.insert(SchemaFloatAttrRule {
            element: element_name.to_string(),
            attr: expand_generated_qname(attr_qname, namespaces, true),
            kind,
        });
    }
}

fn collect_type_float_text(
    ty: &serde_json::Value,
    type_by_name: &BTreeMap<&str, &serde_json::Value>,
    element_name: &str,
    texts: &mut BTreeSet<SchemaFloatTextRule>,
) {
    if let Some(base_name) = ty.get("BaseClass").and_then(serde_json::Value::as_str)
        && let Some(base_ty) = type_by_name.get(base_name)
    {
        collect_type_float_text(base_ty, type_by_name, element_name, texts);
    }

    if !ty
        .get("IsLeafText")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return;
    }
    let Some(kind) = leaf_text_float_kind(ty) else {
        return;
    };
    texts.insert(SchemaFloatTextRule {
        element: element_name.to_string(),
        kind,
    });
}

fn leaf_text_float_kind(ty: &serde_json::Value) -> Option<SchemaFloatKind> {
    ty.get("Validators")
        .and_then(serde_json::Value::as_array)
        .and_then(|validators| {
            validators.iter().find_map(|validator| {
                if validator.get("Name").and_then(serde_json::Value::as_str)
                    != Some("NumberValidator")
                {
                    return None;
                }
                validator.get("Type").and_then(serde_json::Value::as_str)
            })
        })
        .or_else(|| {
            ty.get("Name")
                .and_then(serde_json::Value::as_str)
                .and_then(|name| name.split_once('/').map(|(type_name, _)| type_name))
        })
        .and_then(float_kind)
}

fn float_kind(type_name: &str) -> Option<SchemaFloatKind> {
    Some(match type_name {
        "SingleValue" | "xsd:float" => SchemaFloatKind::Single,
        "DoubleValue" | "xsd:double" | "cx:CT_NumericValue" => SchemaFloatKind::Double,
        _ => return None,
    })
}

fn expand_generated_qname(
    qname: &str,
    namespaces: &BTreeMap<String, String>,
    is_attr: bool,
) -> String {
    let element_qname = qname.rsplit('/').next().unwrap_or(qname);
    let Some((prefix, local_name)) = element_qname.split_once(':') else {
        return element_qname.to_string();
    };
    if prefix.is_empty() && is_attr {
        return local_name.to_string();
    }
    let Some(uri) = namespaces.get(prefix) else {
        return element_qname.to_string();
    };
    format!("{{{uri}}}{local_name}")
}

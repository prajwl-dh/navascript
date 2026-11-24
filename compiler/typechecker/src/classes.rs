//! Class collection, shape resolution and method lookup.

use std::collections::BTreeMap;

use nava_ast::{ClassDecl, ClassMember, ItemKind};
use nava_symbol::Type;

use super::checker::Checker;
use super::types::{Subst, resolve_named_type, union_layout_error};

/// Checked shape of a class, including inherited members.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassInfo {
    pub fields: Vec<(String, Type)>,
    pub methods: BTreeMap<String, Type>,
    pub base: Option<String>,
    pub ctor_params: Vec<Type>,
    pub has_ctor: bool,
    /// Class whose constructor function should be invoked by `new`/`super(...)`.
    pub ctor_owner: Option<String>,
    /// Methods declared directly on this class (not inherited copies).
    pub own_methods: Vec<String>,
}

impl Checker<'_> {
    /// Register class names, then resolve their members (base classes first).
    pub(super) fn collect_classes(&mut self) {
        for item in &self.program.items {
            if let ItemKind::Class(decl) = item.kind.unwrap_export() {
                self.classes.insert(
                    decl.name.name.clone(),
                    ClassInfo {
                        fields: Vec::new(),
                        methods: std::collections::BTreeMap::new(),
                        base: decl.base.as_ref().map(|base| base.name.clone()),
                        ctor_params: Vec::new(),
                        has_ctor: false,
                        ctor_owner: None,
                        own_methods: Vec::new(),
                    },
                );
            }
        }
        let names: Vec<String> = self.classes.keys().cloned().collect();
        for name in &names {
            self.ensure_class(name);
        }
        // A second pass resolves fields whose type names another class that was
        // still empty during the first pass.
        for name in &names {
            let decl = self
                .program
                .items
                .iter()
                .find_map(|item| match item.kind.unwrap_export() {
                    ItemKind::Class(decl) if &decl.name.name == name => Some(decl),
                    _ => None,
                })
                .expect("registered class has a declaration")
                .clone();
            let info = self.resolve_class(&decl);
            self.classes.insert(name.clone(), info);
        }
    }

    fn ensure_class(&mut self, name: &str) {
        if self.class_resolved.contains(name) {
            return;
        }
        let decl = self
            .program
            .items
            .iter()
            .find_map(|item| match item.kind.unwrap_export() {
                ItemKind::Class(decl) if decl.name.name == name => Some(decl.clone()),
                _ => None,
            });
        let Some(decl) = decl else {
            return;
        };
        if let Some(base) = &decl.base {
            if self.classes.contains_key(&base.name) {
                self.ensure_class(&base.name);
            } else {
                self.error(format!("unknown base class '{}'", base.name), base.span);
            }
        }
        let info = self.resolve_class(&decl);
        self.classes.insert(name.to_owned(), info);
        self.class_resolved.insert(name.to_owned());
    }

    fn resolve_class(&mut self, decl: &ClassDecl) -> ClassInfo {
        let subst = Subst::new();
        let mut fields = decl
            .base
            .as_ref()
            .and_then(|base| self.classes.get(&base.name))
            .map(|info| info.fields.clone())
            .unwrap_or_default();
        let mut methods: std::collections::BTreeMap<String, Type> = decl
            .base
            .as_ref()
            .and_then(|base| self.classes.get(&base.name))
            .map(|info| info.methods.clone())
            .unwrap_or_default();
        let mut ctor_params = Vec::new();
        let mut ctor_has_body = false;
        let mut own_methods = Vec::new();
        for member in &decl.members {
            match member {
                ClassMember::Field { name, ty } => {
                    let field_ty = ty
                        .as_ref()
                        .map(|annotation| self.resolve(&annotation.expr, &subst, name.span))
                        .unwrap_or(Type::Unknown);
                    if let Some(message) = union_layout_error(&field_ty, &self.aliases) {
                        self.error(message, name.span);
                    }
                    if let Some(existing) = fields
                        .iter_mut()
                        .find(|(field_name, _)| field_name == &name.name)
                    {
                        existing.1 = field_ty;
                    } else {
                        fields.push((name.name.clone(), field_ty));
                    }
                }
                ClassMember::Constructor { params, .. } => {
                    ctor_has_body = true;
                    ctor_params = params
                        .iter()
                        .map(|param| {
                            param
                                .ty
                                .as_ref()
                                .map(|annotation| {
                                    self.resolve(&annotation.expr, &subst, param.span)
                                })
                                .unwrap_or(Type::Unknown)
                        })
                        .collect();
                }
                ClassMember::Method {
                    name,
                    params,
                    return_ty,
                    ..
                } => {
                    let param_types = params
                        .iter()
                        .map(|param| {
                            param
                                .ty
                                .as_ref()
                                .map(|annotation| {
                                    self.resolve(&annotation.expr, &subst, param.span)
                                })
                                .unwrap_or(Type::Unknown)
                        })
                        .collect();
                    let ret = return_ty
                        .as_ref()
                        .map(|annotation| self.resolve(&annotation.expr, &subst, name.span))
                        .unwrap_or(Type::Unknown);
                    methods.insert(
                        name.name.clone(),
                        Type::Function {
                            params: param_types,
                            ret: Box::new(ret),
                        },
                    );
                    own_methods.push(name.name.clone());
                }
            }
        }
        let (ctor_params, has_ctor, ctor_owner) = if ctor_has_body {
            (ctor_params, true, Some(decl.name.name.clone()))
        } else if let Some(base) = decl
            .base
            .as_ref()
            .and_then(|base| self.classes.get(&base.name))
            .filter(|info| info.has_ctor)
        {
            (base.ctor_params.clone(), true, base.ctor_owner.clone())
        } else {
            (ctor_params, false, None)
        };
        ClassInfo {
            fields,
            methods,
            base: decl.base.as_ref().map(|base| base.name.clone()),
            ctor_params,
            has_ctor,
            ctor_owner,
            own_methods,
        }
    }

    /// Type-check every method and constructor body.
    /// The nominal instance type of a class.
    pub(super) fn instance_type(&self, name: &str) -> Type {
        self.classes
            .get(name)
            .map(|info| Type::ClassInstance {
                name: name.to_owned(),
                fields: info.fields.clone(),
            })
            .unwrap_or(Type::Error)
    }

    /// The class identity carried by a nominal instance type. There is no
    /// structural fallback: a receiver without class identity cannot dispatch
    /// a method, so the checker diagnoses it instead of guessing.
    pub(super) fn class_of(&self, ty: &Type) -> Option<String> {
        let resolved = resolve_named_type(ty, &self.aliases);
        match resolved {
            Type::ClassInstance { name, .. } => Some(name),
            _ => None,
        }
    }

    /// Resolve a method through the base-class chain, returning its mangled
    /// call target and function type.
    pub(super) fn lookup_method(&self, class: &str, method: &str) -> Option<(String, Type)> {
        let mut current = Some(class.to_owned());
        while let Some(name) = current {
            let info = self.classes.get(&name)?;
            if info.own_methods.iter().any(|own| own == method) {
                if let Some(ty) = info.methods.get(method) {
                    return Some((format!("{name}_{method}"), ty.clone()));
                }
            }
            current = info.base.clone();
        }
        None
    }
}

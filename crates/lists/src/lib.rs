//! Built-in sensitive categories and the domain lists they use. The lists are compiled in from
//! `lists/*.txt` as lz4 blobs and expanded into a set the first time one is asked about.

mod bundled;
mod categories;
#[cfg(test)]
mod tests;

use std::collections::HashSet;

use closecall_core::ipc::CategoryInfo;
use closecall_core::matcher::CategoryIndex;
use closecall_core::{CategoryDef, CategoryId};

#[derive(Debug, Clone)]
pub struct Lists {
    categories: Vec<CategoryDef>,
}

impl Lists {
    pub fn bundled() -> Self {
        Self {
            categories: categories::BUILTINS
                .iter()
                .map(categories::Builtin::to_def)
                .collect(),
        }
    }

    /// A user category with a built-in id extends the built-in (union of every field, the
    /// user's name wins); any other id becomes a new category.
    pub fn with_user_categories(mut self, user: &[CategoryDef]) -> Self {
        for def in user {
            let index = match self.categories.iter().position(|c| c.id == def.id) {
                Some(index) => index,
                None => {
                    self.categories.push(CategoryDef {
                        id: def.id.clone(),
                        ..CategoryDef::default()
                    });
                    self.categories.len() - 1
                }
            };
            merge(&mut self.categories[index], def);
        }
        self
    }

    /// Built-ins first, in a fixed order, then user categories. `domain_count` adds up list
    /// entries without removing overlap between lists.
    pub fn infos(&self) -> Vec<CategoryInfo> {
        self.categories
            .iter()
            .map(|def| {
                let sources: Vec<_> = def
                    .lists
                    .iter()
                    .filter_map(|id| bundled::manifest_entry(id))
                    .map(bundled::ManifestEntry::source)
                    .collect();
                CategoryInfo {
                    id: def.id.clone(),
                    builtin: categories::find(&def.id.0).is_some(),
                    default_enabled: Self::default_enabled(&def.id),
                    domain_count: def.domains.len() as u32
                        + sources.iter().map(|source| source.entries).sum::<u32>(),
                    app_count: def.apps.len() as u32,
                    keyword_count: def.title_keywords.len() as u32,
                    sources,
                }
            })
            .collect()
    }

    pub fn builtin_ids() -> Vec<CategoryId> {
        categories::BUILTINS
            .iter()
            .map(|builtin| CategoryId(builtin.id.to_owned()))
            .collect()
    }

    /// False only for the opt-in built-ins (financial, dating, gambling). A category the user
    /// defined is on, since defining it is the opt-in.
    pub fn default_enabled(id: &CategoryId) -> bool {
        categories::find(&id.0).is_none_or(|builtin| builtin.default_enabled)
    }
}

impl CategoryIndex for Lists {
    fn category(&self, id: &CategoryId) -> Option<&CategoryDef> {
        self.categories.iter().find(|def| def.id == *id)
    }

    fn list_contains(&self, list_id: &str, host: &str) -> bool {
        bundled::domains(list_id).is_some_and(|domains| contains_host(domains, host))
    }
}

/// The walk stops at the registrable domain, so an entry that is a public suffix (such as
/// `blogspot.com`) never covers every site registered under it.
fn contains_host(domains: &HashSet<Box<str>>, host: &str) -> bool {
    let host = host.strip_suffix('.').unwrap_or(host);
    let shortest = psl::domain_str(host).map_or(host.len(), str::len);
    let mut candidate = host;
    loop {
        if domains.contains(candidate) {
            return true;
        }
        match candidate.split_once('.') {
            Some((_, parent)) if parent.len() >= shortest => candidate = parent,
            _ => return false,
        }
    }
}

fn merge(into: &mut CategoryDef, from: &CategoryDef) {
    if from.name.is_some() {
        into.name.clone_from(&from.name);
    }
    union(&mut into.domains, &from.domains);
    union(&mut into.lists, &from.lists);
    union(&mut into.apps, &from.apps);
    union(&mut into.title_keywords, &from.title_keywords);
    union(&mut into.file_globs, &from.file_globs);
}

fn union<T: PartialEq + Clone>(into: &mut Vec<T>, from: &[T]) {
    for item in from {
        if !into.contains(item) {
            into.push(item.clone());
        }
    }
}

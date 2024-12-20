use std::ops::Range;

use anyhow::{bail, Result};
use bon::bon;

use crate::analysis::data::{self, RelocationResult, SymbolCandidate};

use super::{
    module::Module,
    section::SectionKind,
    symbol::{SymBss, SymData, SymbolMaps},
};

pub struct Program<'a> {
    modules: Vec<Module<'a>>,
    symbol_maps: SymbolMaps,
    // Indices in modules vec above
    main: usize,
    overlays: Range<usize>,
    autoloads: Range<usize>,
}

#[bon]
impl<'a> Program<'a> {
    pub fn new(main: Module<'a>, overlays: Vec<Module<'a>>, autoloads: Vec<Module<'a>>, symbol_maps: SymbolMaps) -> Self {
        let mut modules = vec![main];
        let main = 0;

        modules.extend(overlays);
        let overlays = (main + 1)..modules.len();

        modules.extend(autoloads);
        let autoloads = overlays.end..modules.len();

        Self { modules, symbol_maps, main, overlays, autoloads }
    }

    #[builder]
    pub fn analyze_cross_references(&mut self, allow_unknown_function_calls: bool) -> Result<()> {
        for module_index in 0..self.modules.len() {
            let RelocationResult { relocations, external_symbols } = data::analyze_external_references()
                .modules(&self.modules)
                .module_index(module_index)
                .symbol_maps(&mut self.symbol_maps)
                .allow_unknown_function_calls(allow_unknown_function_calls)
                .call()?;

            self.modules[module_index].relocations_mut().extend(relocations)?;

            for symbol in external_symbols {
                match symbol.candidates.len() {
                    0 => {
                        log::error!("There should be at least one symbol candidate");
                        bail!("There should be at least one symbol candidate");
                    }
                    1 => {
                        let SymbolCandidate { module_index, section_index } = symbol.candidates[0];
                        let section_kind = self.modules[module_index].sections().get(section_index).kind();
                        let name = format!("{}{:08x}", self.modules[module_index].default_data_prefix, symbol.address);
                        let symbol_map = self.symbol_maps.get_mut(self.modules[module_index].kind());
                        match section_kind {
                            SectionKind::Code => {} // Function symbol, already verified to exist
                            SectionKind::Data => {
                                symbol_map.add_data(Some(name), symbol.address, SymData::Any)?;
                            }
                            SectionKind::Bss => {
                                symbol_map.add_bss(Some(name), symbol.address, SymBss { size: None })?;
                            }
                        }
                    }
                    _ => {
                        for SymbolCandidate { module_index, section_index } in symbol.candidates {
                            let section_kind = self.modules[module_index].sections().get(section_index).kind();
                            let name = format!("{}{:08x}", self.modules[module_index].default_data_prefix, symbol.address);
                            let symbol_map = self.symbol_maps.get_mut(self.modules[module_index].kind());
                            match section_kind {
                                SectionKind::Code => {} // Function symbol, already verified to exist
                                SectionKind::Data => {
                                    symbol_map.add_ambiguous_data(Some(name), symbol.address, SymData::Any)?;
                                }
                                SectionKind::Bss => {
                                    symbol_map.add_ambiguous_bss(Some(name), symbol.address, SymBss { size: None })?;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn main(&self) -> &Module {
        &self.modules[self.main]
    }

    pub fn overlays(&self) -> &[Module] {
        &self.modules[self.overlays.clone()]
    }

    pub fn autoloads(&self) -> &[Module] {
        &self.modules[self.autoloads.clone()]
    }

    pub fn module(&self, index: usize) -> &Module {
        &self.modules[index]
    }

    pub fn module_mut(&'a mut self, index: usize) -> &mut Module {
        &mut self.modules[index]
    }

    pub fn num_modules(&self) -> usize {
        self.modules.len()
    }

    pub fn symbol_maps(&self) -> &SymbolMaps {
        &self.symbol_maps
    }
}

pub struct ExternalModules<'a> {
    before: &'a mut [Module<'a>],
    after: &'a mut [Module<'a>],
    module_index: usize,
}

impl<'a> ExternalModules<'a> {
    pub fn get(&self, index: usize) -> &Module {
        if index < self.module_index {
            &self.before[index]
        } else {
            &self.after[index - self.module_index]
        }
    }

    pub fn get_mut(&'a mut self, index: usize) -> &mut Module {
        if index < self.module_index {
            &mut self.before[index]
        } else {
            &mut self.after[index - self.module_index]
        }
    }

    pub unsafe fn get_mut_ptr(&'a mut self, index: usize) -> *mut Module {
        if index < self.module_index {
            &mut self.before[index]
        } else {
            &mut self.after[index - self.module_index]
        }
    }

    pub fn len(&self) -> usize {
        self.module_index + self.after.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Module> {
        self.before.iter().chain(self.after.iter())
    }
}

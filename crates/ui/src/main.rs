//! Main entry point for the Leitor de Cupons Fiscais application.
//!
//! This module sets up the eframe + egui UI for the receipt scanning app.

use anyhow::Result;
use db::{CupomCompleto, CupomResumo, Receita, Repositorio};
use eframe::egui;
use receipt_core::{
    calcular_responsabilidades, calcular_totais, validar_total, CoreError, Grupo, Item,
    ItemClassificado, Pagador, Totais,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::info;

/// Modo de operação da aplicação
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModoOperacao {
    #[default]
    /// Seleção de imagem / pipeline OCR+LLM
    Automatico,
    /// Entrada manual de cupom
    Manual,
}

/// Estado de um item em edição
#[derive(Debug, Clone)]
pub struct ItemEmEdicao {
    descricao: String,
    quantidade: String,
    preco_unitario: String,
    preco_total: String,
    grupo: Option<Grupo>,
    /// Se o item é novo (ainda não adicionado à lista)
    novo: bool,
    /// Índice do item na lista (se editando)
    indice: Option<usize>,
}

impl Default for ItemEmEdicao {
    fn default() -> Self {
        Self {
            descricao: String::new(),
            quantidade: "1".to_string(),
            preco_unitario: String::new(),
            preco_total: String::new(),
            grupo: None,
            novo: true,
            indice: None,
        }
    }
}

impl ItemEmEdicao {
    fn limpar(&mut self) {
        self.descricao.clear();
        self.quantidade = "1".to_string();
        self.preco_unitario.clear();
        self.preco_total.clear();
        self.grupo = None;
        self.novo = true;
        self.indice = None;
    }

    fn carregar_de_item(&mut self, item: &ItemClassificado, indice: usize) {
        self.descricao = item.item.descricao.clone();
        self.quantidade = item.item.quantidade.to_string();
        self.preco_unitario = item.item.preco_unitario.to_string();
        self.preco_total = item.item.preco_total.to_string();
        self.grupo = Some(item.grupo);
        self.novo = false;
        self.indice = Some(indice);
    }

    fn validar(&self) -> Result<(), String> {
        if self.descricao.trim().is_empty() {
            return Err("Descrição não pode ser vazia".to_string());
        }
        let qtd: f64 = self
            .quantidade
            .parse()
            .map_err(|_| "Quantidade inválida".to_string())?;
        if qtd <= 0.0 {
            return Err("Quantidade deve ser positiva".to_string());
        }
        let pu: f64 = self
            .preco_unitario
            .parse()
            .map_err(|_| "Preço unitário inválido".to_string())?;
        if pu < 0.0 {
            return Err("Preço unitário não pode ser negativo".to_string());
        }
        let pt: f64 = self
            .preco_total
            .parse()
            .map_err(|_| "Preço total inválido".to_string())?;
        if pt < 0.0 {
            return Err("Preço total não pode ser negativo".to_string());
        }
        if self.grupo.is_none() {
            return Err("Grupo deve ser selecionado".to_string());
        }
        Ok(())
    }

    fn para_item_classificado(&self) -> ItemClassificado {
        let quantidade: f64 = self.quantidade.parse().unwrap_or(1.0);
        let preco_unitario: f64 = self.preco_unitario.parse().unwrap_or(0.0);
        let preco_total: f64 = self.preco_total.parse().unwrap_or(0.0);

        ItemClassificado {
            item: Item {
                descricao: self.descricao.trim().to_string(),
                quantidade,
                preco_unitario,
                preco_total,
            },
            grupo: self.grupo.unwrap_or(Grupo::Casa),
        }
    }
}

/// Estado global da aplicação
#[derive(Default)]
pub struct AppState {
    /// Modo atual
    pub modo: ModoOperacao,
    /// Caminho da imagem selecionada (modo automático)
    pub imagem_selecionada: Option<PathBuf>,
    /// Resultado do OCR
    pub texto_ocr: Option<String>,
    /// Itens extraídos pelo LLM
    pub itens: Vec<Item>,
    /// Itens classificados
    pub itens_classificados: Vec<ItemClassificado>,
    /// Totais calculados
    pub totais: Totais,
    /// Erro atual
    pub erro: Option<String>,
    /// Pipeline em andamento
    pub pipeline_ativo: bool,
    /// Progresso do pipeline (0.0 a 1.0)
    pub progresso: f32,
    /// Conexão com banco de dados
    pub db: Option<Arc<Mutex<Repositorio>>>,
    /// Caminho do banco de dados
    pub db_path: PathBuf,
    /// Dados do cupom em edição (modo manual)
    pub cupom_manual: CupomEmEdicao,
    /// Item sendo editado
    pub item_em_edicao: ItemEmEdicao,
    /// Lista de cupons salvos
    pub historico: Vec<CupomResumo>,
    /// Cupom selecionado no histórico
    pub cupom_selecionado: Option<CupomCompleto>,
    /// Mostrando detalhes do cupom
    pub mostrando_detalhes: bool,
}

/// Dados do cupom sendo editado manualmente
#[derive(Debug, Clone, Default)]
pub struct CupomEmEdicao {
    estabelecimento: String,
    data_compra: String,
    total_cupom: String,
    imagem_path: String,
    pagador: String, // "igor" ou "renan"
}

impl CupomEmEdicao {
    fn limpar(&mut self) {
        self.estabelecimento.clear();
        self.data_compra.clear();
        self.total_cupom.clear();
        self.imagem_path.clear();
        self.pagador = "igor".to_string(); // Default
    }

    fn validar(&self, itens: &[Item]) -> Result<(), String> {
        if self.estabelecimento.trim().is_empty() {
            return Err("Estabelecimento não pode ser vazio".to_string());
        }
        if self.data_compra.trim().is_empty() {
            return Err("Data da compra não pode ser vazia".to_string());
        }
        let total: f64 = self
            .total_cupom
            .parse()
            .map_err(|_| "Total do cupom inválido".to_string())?;
        if total < 0.0 {
            return Err("Total do cupom não pode ser negativo".to_string());
        }
        if itens.is_empty() {
            return Err("Adicione pelo menos um item".to_string());
        }
        Ok(())
    }
}

/// Implementação da aplicação eframe
impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Inicializar DB se necessário
        if self.db.is_none() {
            self.inicializar_db();
        }

        // Cabeçalho com seleção de modo
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Leitor de Cupons Fiscais");
                ui.separator();
                egui::ComboBox::from_id_salt("modo_operacao")
                    .selected_text(match self.modo {
                        ModoOperacao::Automatico => "📷 Automático (OCR + LLM)",
                        ModoOperacao::Manual => "✏️ Manual",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.modo,
                            ModoOperacao::Automatico,
                            "📷 Automático (OCR + LLM)",
                        );
                        ui.selectable_value(&mut self.modo, ModoOperacao::Manual, "✏️ Manual");
                    });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(ref path) = self.imagem_selecionada {
                        ui.label(format!("Imagem: {}", path.display()));
                    }
                });
            });
        });

        // Área principal
        egui::CentralPanel::default().show(ctx, |ui| match self.modo {
            ModoOperacao::Automatico => self.ui_automatico(ui),
            ModoOperacao::Manual => self.ui_manual(ui),
        });

        // Painel lateral com histórico
        egui::SidePanel::right("historico")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Histórico");
                ui.separator();
                if self.historico.is_empty() {
                    ui.label("Nenhum cupom salvo ainda.");
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let mut cupom_para_carregar: Option<i64> = None;

                        for cupom in &self.historico {
                            let selecionado =
                                self.cupom_selecionado.as_ref().map(|c| c.id).unwrap_or(-1)
                                    == cupom.id;
                            if ui
                                .selectable_label(
                                    selecionado,
                                    format!(
                                        "{} - {} - R$ {:.2}",
                                        cupom.data_compra, cupom.estabelecimento, cupom.total_cupom
                                    ),
                                )
                                .clicked()
                            {
                                cupom_para_carregar = Some(cupom.id);
                            }
                        }

                        // Processar o cupom selecionado fora do loop
                        if let Some(id) = cupom_para_carregar {
                            self.carregar_cupom(id);
                        }
                    });
                }
                ui.add_space(10.0);
                if ui.button("Atualizar lista").clicked() {
                    self.atualizar_historico();
                }
            });
    }
}

impl AppState {
    fn inicializar_db(&mut self) {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        self.db_path = home.join("cupons.db");

        match Repositorio::new(&self.db_path) {
            Ok(repo) => {
                self.db = Some(Arc::new(Mutex::new(repo)));
                self.atualizar_historico();
                info!("Banco de dados inicializado em {:?}", self.db_path);
            }
            Err(e) => {
                self.erro = Some(format!("Erro ao inicializar banco: {}", e));
                tracing::error!("Erro ao inicializar banco: {}", e);
            }
        }
    }

    fn atualizar_historico(&mut self) {
        if let Some(ref db) = self.db {
            let db_guard = db.lock().unwrap();
            match Repositorio::listar_cupons(&db_guard.conn) {
                Ok(cupons) => self.historico = cupons,
                Err(e) => {
                    self.erro = Some(format!("Erro ao listar cupons: {}", e));
                    tracing::error!("Erro ao listar cupons: {}", e);
                }
            }
        }
    }

    fn carregar_cupom(&mut self, id: i64) {
        if let Some(ref db) = self.db {
            let db_guard = db.lock().unwrap();
            if let Ok(Some(cupom)) = Repositorio::buscar_cupom(&db_guard.conn, id) {
                let itens = Repositorio::buscar_itens_por_receipt_id(&db_guard.conn, id)
                    .unwrap_or_default();
                let mut cupom_completo = cupom;
                cupom_completo.itens = itens;
                self.cupom_selecionado = Some(cupom_completo);
                self.mostrando_detalhes = true;
            }
        }
    }

    fn ui_automatico(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            if self.imagem_selecionada.is_none() {
                ui.label("Selecione uma imagem de cupom fiscal para começar");
                ui.add_space(20.0);
                if ui.button("Selecionar Cupom").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("image", &["png", "jpg", "jpeg"])
                        .pick_file()
                    {
                        self.imagem_selecionada = Some(path);
                        self.erro = None;
                        self.texto_ocr = None;
                        self.itens.clear();
                        self.itens_classificados.clear();
                        self.totais = Totais::default();
                        self.pipeline_ativo = false;
                        self.progresso = 0.0;
                    }
                }
            } else {
                if self.pipeline_ativo {
                    ui.label("Processando cupom...");
                    ui.add(egui::ProgressBar::new(self.progresso).show_percentage());
                }

                if let Some(ref erro) = self.erro {
                    ui.colored_label(egui::Color32::RED, format!("Erro: {}", erro));
                }

                if !self.itens_classificados.is_empty() {
                    egui::Grid::new("itens_grid_auto")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.strong("Descrição");
                            ui.strong("Qtd");
                            ui.strong("Preço Unit.");
                            ui.strong("Preço Total");
                            ui.strong("Grupo");
                            ui.end_row();

                            for item in &mut self.itens_classificados {
                                ui.label(&item.item.descricao);
                                ui.label(format!("{:.0}", item.item.quantidade));
                                ui.label(format!("R$ {:.2}", item.item.preco_unitario));
                                ui.label(format!("R$ {:.2}", item.item.preco_total));

                                egui::ComboBox::from_id_salt(item.item.descricao.clone())
                                    .selected_text(format!("{}", item.grupo))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut item.grupo, Grupo::Casa, "Casa");
                                        ui.selectable_value(&mut item.grupo, Grupo::Igor, "Igor");
                                        ui.selectable_value(&mut item.grupo, Grupo::Renan, "Renan");
                                    });
                                ui.end_row();
                            }
                        });

                    self.totais = calcular_totais(&self.itens_classificados);

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    egui::Grid::new("totais_grid_auto").show(ui, |ui| {
                        ui.strong("Casa:");
                        ui.label(format!("R$ {:.2}", self.totais.casa));
                        ui.end_row();
                        ui.strong("Igor:");
                        ui.label(format!("R$ {:.2}", self.totais.igor));
                        ui.end_row();
                        ui.strong("Renan:");
                        ui.label(format!("R$ {:.2}", self.totais.renan));
                        ui.end_row();
                        ui.strong("Geral:");
                        ui.label(format!("R$ {:.2}", self.totais.geral));
                        ui.end_row();
                    });

                    ui.add_space(20.0);

                    ui.horizontal(|ui| {
                        if ui.button("Salvar").clicked() {
                            let _todos_com_grupo = self
                                .itens_classificados
                                .iter()
                                .all(|i| i.grupo != Grupo::Casa || true);
                            info!("Salvar cupom automático - placeholder");
                        }
                        if ui.button("Cancelar").clicked() {
                            self.imagem_selecionada = None;
                            self.itens.clear();
                            self.itens_classificados.clear();
                            self.totais = Totais::default();
                        }
                    });
                } else {
                    ui.label("Nenhum item extraído ainda.");
                }
            }
        });
    }

    fn ui_manual(&mut self, ui: &mut egui::Ui) {
        // Mostrar detalhes do cupom selecionado no histórico
        if self.mostrando_detalhes {
            if let Some(ref cupom) = self.cupom_selecionado {
                // Calcular responsabilidades
                let pagador = match cupom.receita.pagador.as_str() {
                    "renan" => Pagador::Renan,
                    _ => Pagador::Igor,
                };
                let responsabilidades = calcular_responsabilidades(
                    &calcular_totais(&cupom.itens),
                    pagador,
                    cupom.receita.total_cupom,
                );

                ui.heading("Detalhes do Cupom");
                ui.separator();
                egui::Grid::new("detalhes_cupom").show(ui, |ui| {
                    ui.strong("Estabelecimento:");
                    ui.label(&cupom.receita.estabelecimento);
                    ui.end_row();
                    ui.strong("Data:");
                    ui.label(&cupom.receita.data_compra);
                    ui.end_row();
                    ui.strong("Total do Cupom:");
                    ui.label(format!("R$ {:.2}", cupom.receita.total_cupom));
                    ui.end_row();
                    ui.strong("Pagador:");
                    ui.label(match pagador {
                        Pagador::Igor => "Igor",
                        Pagador::Renan => "Renan",
                    });
                    ui.end_row();
                });
                ui.add_space(10.0);

                egui::Grid::new("itens_detalhes")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Descrição");
                        ui.strong("Qtd");
                        ui.strong("Preço Unit.");
                        ui.strong("Preço Total");
                        ui.strong("Grupo");
                        ui.end_row();

                        for item in &cupom.itens {
                            ui.label(&item.item.descricao);
                            ui.label(format!("{:.0}", item.item.quantidade));
                            ui.label(format!("R$ {:.2}", item.item.preco_unitario));
                            ui.label(format!("R$ {:.2}", item.item.preco_total));
                            ui.label(format!("{}", item.grupo));
                            ui.end_row();
                        }
                    });

                let totais = calcular_totais(&cupom.itens);
                ui.add_space(10.0);
                ui.separator();
                egui::Grid::new("totais_detalhes").show(ui, |ui| {
                    ui.strong("Casa:");
                    ui.label(format!("R$ {:.2}", totais.casa));
                    ui.end_row();
                    ui.strong("Igor (pessoal):");
                    ui.label(format!("R$ {:.2}", totais.igor));
                    ui.end_row();
                    ui.strong("Renan (pessoal):");
                    ui.label(format!("R$ {:.2}", totais.renan));
                    ui.end_row();
                    ui.strong("Geral:");
                    ui.label(format!("R$ {:.2}", totais.geral));
                    ui.end_row();
                });

                ui.add_space(10.0);
                ui.separator();
                ui.strong("Responsabilidades (Casa 50/50, centavo para quem não pagou):");
                ui.add_space(5.0);
                egui::Grid::new("resp_detalhes").show(ui, |ui| {
                    ui.strong("Igor (responsabilidade):");
                    ui.label(format!("R$ {:.2}", responsabilidades.igor));
                    ui.end_row();
                    ui.strong("Renan (responsabilidade):");
                    ui.label(format!("R$ {:.2}", responsabilidades.renan));
                    ui.end_row();
                    ui.strong("Geral (responsabilidade):");
                    ui.label(format!("R$ {:.2}", responsabilidades.geral));
                    ui.end_row();
                    ui.strong("Pagador:");
                    ui.label(format!(
                        "{} (pagou R$ {:.2})",
                        match responsabilidades.pagador {
                            Pagador::Igor => "Igor",
                            Pagador::Renan => "Renan",
                        },
                        responsabilidades.valor_pago
                    ));
                    ui.end_row();
                    ui.strong("Saldo Igor:");
                    ui.colored_label(
                        if responsabilidades.saldo_igor >= 0.0 {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::RED
                        },
                        format!(
                            "{} R$ {:.2}",
                            if responsabilidades.saldo_igor >= 0.0 {
                                "↑ crédito"
                            } else {
                                "↓ débito"
                            },
                            responsabilidades.saldo_igor.abs()
                        ),
                    );
                    ui.end_row();
                    ui.strong("Saldo Renan:");
                    ui.colored_label(
                        if responsabilidades.saldo_renan >= 0.0 {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::RED
                        },
                        format!(
                            "{} R$ {:.2}",
                            if responsabilidades.saldo_renan >= 0.0 {
                                "↑ crédito"
                            } else {
                                "↓ débito"
                            },
                            responsabilidades.saldo_renan.abs()
                        ),
                    );
                    ui.end_row();
                });

                ui.add_space(20.0);
                if ui.button("Fechar detalhes").clicked() {
                    self.mostrando_detalhes = false;
                    self.cupom_selecionado = None;
                }
                return;
            }
        }

        // Formulário de edição do cupom
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Novo Cupom (Manual)");
            ui.separator();

            egui::Grid::new("cupom_form")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.strong("Estabelecimento:");
                    ui.text_edit_singleline(&mut self.cupom_manual.estabelecimento);
                    ui.end_row();

                    ui.strong("Data da compra (YYYY-MM-DD):");
                    ui.text_edit_singleline(&mut self.cupom_manual.data_compra);
                    ui.end_row();

                    ui.strong("Total do cupom (R$):");
                    ui.text_edit_singleline(&mut self.cupom_manual.total_cupom);
                    ui.end_row();

                    ui.strong("Quem pagou:");
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("pagador_cupom")
                            .selected_text(self.cupom_manual.pagador.clone())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.cupom_manual.pagador,
                                    "igor".to_string(),
                                    "Igor",
                                );
                                ui.selectable_value(
                                    &mut self.cupom_manual.pagador,
                                    "renan".to_string(),
                                    "Renan",
                                );
                            });
                    });
                    ui.end_row();

                    ui.strong("Caminho da imagem (opcional):");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.cupom_manual.imagem_path);
                        if ui.button("Selecionar").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("image", &["png", "jpg", "jpeg"])
                                .pick_file()
                            {
                                self.cupom_manual.imagem_path = path.to_string_lossy().to_string();
                            }
                        }
                    });
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // Seção de itens
            ui.horizontal(|ui| {
                ui.strong("Itens do Cupom");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕ Adicionar item").clicked() {
                        self.item_em_edicao.limpar();
                    }
                });
            });

            ui.add_space(5.0);

            // Formulário de item
            egui::Grid::new("item_form")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.strong("Descrição:");
                    ui.text_edit_singleline(&mut self.item_em_edicao.descricao);
                    ui.end_row();

                    ui.strong("Quantidade:");
                    ui.text_edit_singleline(&mut self.item_em_edicao.quantidade);
                    ui.end_row();

                    ui.strong("Preço unitário (R$):");
                    ui.text_edit_singleline(&mut self.item_em_edicao.preco_unitario);
                    ui.end_row();

                    ui.strong("Preço total (R$):");
                    ui.text_edit_singleline(&mut self.item_em_edicao.preco_total);
                    ui.end_row();

                    ui.strong("Grupo:");
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("grupo_item")
                            .selected_text(
                                self.item_em_edicao
                                    .grupo
                                    .map(|g| format!("{}", g))
                                    .unwrap_or_else(|| "Selecione...".to_string()),
                            )
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.item_em_edicao.grupo,
                                    Some(Grupo::Casa),
                                    "Casa",
                                );
                                ui.selectable_value(
                                    &mut self.item_em_edicao.grupo,
                                    Some(Grupo::Igor),
                                    "Igor",
                                );
                                ui.selectable_value(
                                    &mut self.item_em_edicao.grupo,
                                    Some(Grupo::Renan),
                                    "Renan",
                                );
                            });
                    });
                    ui.end_row();
                });

            // Botões do item
            ui.horizontal(|ui| {
                if self.item_em_edicao.novo {
                    if ui.button("Adicionar").clicked() {
                        if let Err(e) = self.item_em_edicao.validar() {
                            self.erro = Some(e);
                        } else {
                            let item = self.item_em_edicao.para_item_classificado();
                            self.itens_classificados.push(item);
                            self.item_em_edicao.limpar();
                            self.erro = None;
                        }
                    }
                } else {
                    if ui.button("Salvar alterações").clicked() {
                        if let Err(e) = self.item_em_edicao.validar() {
                            self.erro = Some(e);
                        } else if let Some(indice) = self.item_em_edicao.indice {
                            let item = self.item_em_edicao.para_item_classificado();
                            self.itens_classificados[indice] = item;
                            self.item_em_edicao.limpar();
                            self.erro = None;
                        }
                    }
                    if ui.button("Cancelar").clicked() {
                        self.item_em_edicao.limpar();
                        self.erro = None;
                    }
                }
            });

            // Mostrar erro do item se houver
            if let Some(ref erro) = self.erro {
                ui.colored_label(egui::Color32::RED, erro);
            }

            ui.add_space(10.0);

            // Lista de itens
            if !self.itens_classificados.is_empty() {
                egui::Grid::new("itens_lista").striped(true).show(ui, |ui| {
                    ui.strong("Descrição");
                    ui.strong("Qtd");
                    ui.strong("Preço Unit.");
                    ui.strong("Preço Total");
                    ui.strong("Grupo");
                    ui.strong("");
                    ui.end_row();

                    let mut indice_para_remover: Option<usize> = None;
                    let mut indice_para_editar: Option<usize> = None;

                    for (idx, item) in self.itens_classificados.iter().enumerate() {
                        ui.label(&item.item.descricao);
                        ui.label(format!("{:.0}", item.item.quantidade));
                        ui.label(format!("R$ {:.2}", item.item.preco_unitario));
                        ui.label(format!("R$ {:.2}", item.item.preco_total));
                        ui.label(format!("{}", item.grupo));
                        ui.horizontal(|ui| {
                            if ui.small_button("✏️").clicked() {
                                indice_para_editar = Some(idx);
                            }
                            if ui.small_button("🗑️").clicked() {
                                indice_para_remover = Some(idx);
                            }
                        });
                        ui.end_row();
                    }

                    // Processar remoção após o loop para evitar problemas de empréstimo
                    if let Some(idx) = indice_para_remover {
                        self.itens_classificados.remove(idx);
                    }
                    if let Some(idx) = indice_para_editar {
                        if let Some(item) = self.itens_classificados.get(idx).cloned() {
                            self.item_em_edicao.carregar_de_item(&item, idx);
                        }
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Recalcular totais
                self.totais = calcular_totais(&self.itens_classificados);

                // Mostrar totais
                egui::Grid::new("totais_grid_manual").show(ui, |ui| {
                    ui.strong("Casa:");
                    ui.label(format!("R$ {:.2}", self.totais.casa));
                    ui.end_row();
                    ui.strong("Igor:");
                    ui.label(format!("R$ {:.2}", self.totais.igor));
                    ui.end_row();
                    ui.strong("Renan:");
                    ui.label(format!("R$ {:.2}", self.totais.renan));
                    ui.end_row();
                    ui.strong("Geral:");
                    ui.label(format!("R$ {:.2}", self.totais.geral));
                    ui.end_row();
                });

                ui.add_space(10.0);

                // Validação do total
                let itens_sem_grupo: Vec<Item> = self
                    .itens_classificados
                    .iter()
                    .map(|ic| ic.item.clone())
                    .collect();
                let total_cupom: f64 = self.cupom_manual.total_cupom.parse().unwrap_or(0.0);
                match validar_total(&itens_sem_grupo, total_cupom) {
                    Ok(()) => {
                        ui.colored_label(
                            egui::Color32::GREEN,
                            "✓ Soma dos itens concilia com o total do cupom (diferença ≤ R$ 0,05)",
                        );
                    }
                    Err(CoreError::ValidacaoTotal(e)) => {
                        ui.colored_label(
                            egui::Color32::RED,
                            format!("✗ Diferença: R$ {:.2} (limite: R$ 0,05)", e.0),
                        );
                    }
                    Err(e) => {
                        ui.colored_label(egui::Color32::RED, format!("Erro de validação: {}", e));
                    }
                }

                ui.add_space(20.0);

                // Botões de ação
                ui.horizontal(|ui| {
                    if ui.button("💾 Salvar Cupom").clicked() {
                        self.salvar_cupom_manual();
                    }
                    if ui.button("🔄 Novo Cupom").clicked() {
                        self.novo_cupom_manual();
                    }
                    if ui.button("❌ Cancelar").clicked() {
                        self.novo_cupom_manual();
                    }
                });
            } else {
                ui.label("Nenhum item adicionado ainda. Use 'Adicionar item' para começar.");
            }
        });
    }

    fn salvar_cupom_manual(&mut self) {
        // Validar cupom
        let itens_sem_grupo: Vec<Item> = self
            .itens_classificados
            .iter()
            .map(|ic| ic.item.clone())
            .collect();
        if let Err(e) = self.cupom_manual.validar(&itens_sem_grupo) {
            self.erro = Some(e);
            return;
        }

        // Validar total
        let total_cupom: f64 = self.cupom_manual.total_cupom.parse().unwrap_or(0.0);
        if let Err(e) = validar_total(&itens_sem_grupo, total_cupom) {
            self.erro = Some(format!("Validação falhou: {}", e));
            return;
        }

        // Verificar se todos os itens têm grupo
        if self
            .itens_classificados
            .iter()
            .any(|ic| ic.grupo == Grupo::Casa && ic.item.descricao.is_empty())
        {
            // Esto é só uma verificação básica
        }
        for item in &self.itens_classificados {
            if item.item.descricao.trim().is_empty() {
                self.erro = Some("Todos os itens devem ter descrição".to_string());
                return;
            }
        }

        // Salvar no banco
        if let Some(ref db) = self.db {
            let mut db_guard = db.lock().unwrap();
            let cupom = CupomCompleto {
                id: 0, // 0 = novo cupom, será preenchido pelo banco
                receita: Receita {
                    data_compra: self.cupom_manual.data_compra.clone(),
                    estabelecimento: self.cupom_manual.estabelecimento.clone(),
                    total_cupom,
                    imagem_path: self.cupom_manual.imagem_path.clone(),
                    pagador: self.cupom_manual.pagador.clone(),
                },
                itens: self.itens_classificados.clone(),
            };

            let resultado = Repositorio::salvar_cupom(&mut db_guard.conn, &cupom);

            match resultado {
                Ok(id) => {
                    info!("Cupom salvo com ID {}", id);
                    self.erro = None;
                    // Liberar o lock antes de chamar métodos mutáveis
                    drop(db_guard);
                    self.novo_cupom_manual();
                    self.atualizar_historico();
                }
                Err(e) => {
                    self.erro = Some(format!("Erro ao salvar: {}", e));
                    tracing::error!("Erro ao salvar cupom: {}", e);
                }
            }
        }
    }

    fn novo_cupom_manual(&mut self) {
        self.cupom_manual.limpar();
        self.itens_classificados.clear();
        self.item_em_edicao.limpar();
        self.totais = Totais::default();
        self.erro = None;
        self.cupom_selecionado = None;
        self.mostrando_detalhes = false;
    }
}

/// Função principal da aplicação
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inicializar subscriber de tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(true)
        .with_thread_names(true)
        .init();

    info!("Iniciando Leitor de Cupons Fiscais...");

    // Configurar opções do eframe
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Leitor de Cupons Fiscais")
            .with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };

    // Iniciar aplicação
    eframe::run_native(
        "Leitor de Cupons Fiscais",
        options,
        Box::new(|_cc| Ok(Box::new(AppState::default()))),
    )?;

    Ok(())
}

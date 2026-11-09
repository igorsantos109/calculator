//! Main entry point for the Leitor de Cupons Fiscais application.
//!
//! This module sets up the eframe + egui UI for the receipt scanning app.

use anyhow::Result;
use eframe::egui;
use receipt_core::{calcular_totais, Grupo, Item, ItemClassificado, Totais};
use std::path::PathBuf;
use tracing::info;

/// Estado global da aplicação
#[derive(Default)]
pub struct AppState {
    /// Caminho da imagem selecionada
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
}

/// Implementação da aplicação eframe
impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Cabeçalho
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Leitor de Cupons Fiscais");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(ref path) = self.imagem_selecionada {
                        ui.label(format!("Imagem: {}", path.display()));
                    }
                });
            });
        });

        // Área principal
        egui::CentralPanel::default().show(ctx, |ui| {
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
                            self.itens = Vec::new();
                            self.itens_classificados = Vec::new();
                            self.totais = Totais::default();
                            self.pipeline_ativo = false;
                            self.progresso = 0.0;
                        }
                    }
                } else {
                    // Mostrar progresso se pipeline ativo
                    if self.pipeline_ativo {
                        ui.label("Processando cupom...");
                        ui.add(egui::ProgressBar::new(self.progresso).show_percentage());
                    }

                    // Mostrar erro se houver
                    if let Some(ref erro) = self.erro {
                        ui.colored_label(egui::Color32::RED, format!("Erro: {}", erro));
                    }

                    // Tabela de itens
                    if !self.itens_classificados.is_empty() {
                        egui::Grid::new("itens_grid").striped(true).show(ui, |ui| {
                            // Cabeçalho
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

                                // ComboBox para grupo
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

                        // Recalcular totais
                        self.totais = calcular_totais(&self.itens_classificados);

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Mostrar totais
                        egui::Grid::new("totais_grid").show(ui, |ui| {
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

                        // Botões de ação
                        ui.horizontal(|ui| {
                            if ui.button("Salvar").clicked() {
                                // Validar se todos têm grupo
                                let _todos_com_grupo = self
                                    .itens_classificados
                                    .iter()
                                    .all(|i| i.grupo != Grupo::Casa || true);
                                // Aqui implementaria a lógica de salvar no banco
                                info!("Salvar cupom - placeholder");
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
        });
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
            .with_inner_size([800.0, 600.0]),
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

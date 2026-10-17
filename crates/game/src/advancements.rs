//! Advancements, statistics, the recovery compass and the locator bar (Phase 6).
//!
//! Each advancement is a line of data: tab, key, name, description, icon item and a criterion
//! written as a short rule ("has:diamond", "ev:enchanted", "stat:blocks_mined>=1000"...). The
//! rules are checked once a second (events as they happen), and an earned advancement shows a
//! toast at the top right. L opens the screen with the five tabs and the statistics.

use std::collections::HashSet;

use glam::Vec3;
use voxel::item::ItemStack;

use crate::progression::Event;

/// Tab, key, name, description, icon, rule.
pub type Adv = (u8, &'static str, &'static str, &'static str, &'static str, &'static str);

pub const TABS: [&str; 5] = ["Minecraft", "Nether", "O Fim", "Aventura", "Criação"];

/// The advancements, in the order they show on their tab. Rules:
/// `has:a|b` holds any of the items, `allhas:a,b` holds all, `wear:a|b` wears any,
/// `wearall:a,b` wears all, `ev:token|token` an event happened, `stat:key>=n`,
/// `dim:n` entered a dimension, `biomes:n` visited n overworld biomes, `lvl:n` xp level,
/// `killedall:a,b` killed each at least once, `count:n` advancements earned.
pub const ADVANCEMENTS: &[Adv] = &[
    // Minecraft (story)
    (0, "story/root", "Minecraft", "O coração e a história do jogo", "grass_block", "stat:play_seconds>=1"),
    (0, "story/mine_stone", "Idade da Pedra", "Minere pedra com sua picareta nova", "wooden_pickaxe", "has:cobblestone"),
    (0, "story/upgrade_tools", "Melhorias", "Faça uma picareta melhor", "stone_pickaxe", "has:stone_pickaxe"),
    (0, "story/smelt_iron", "Adquira Hardware", "Funda um lingote de ferro", "iron_ingot", "has:iron_ingot"),
    (0, "story/obtain_armor", "Vista-se", "Proteja-se com uma peça de armadura de ferro", "iron_chestplate", "wear:iron_helmet|iron_chestplate|iron_leggings|iron_boots"),
    (0, "story/lava_bucket", "Coisa Quente", "Encha um balde com lava", "lava_bucket", "has:lava_bucket"),
    (0, "story/iron_tools", "Isso Não é Picareta de Ferro?", "Melhore sua picareta", "iron_pickaxe", "has:iron_pickaxe"),
    (0, "story/deflect_arrow", "Hoje Não, Obrigado", "Desvie um projétil com um escudo", "shield", "ev:blocked"),
    (0, "story/form_obsidian", "Desafio do Balde de Gelo", "Obtenha um bloco de obsidiana", "obsidian", "has:obsidian"),
    (0, "story/mine_diamond", "Diamantes!", "Obtenha diamantes", "diamond", "has:diamond"),
    (0, "story/enter_the_nether", "Precisamos Ir Mais Fundo", "Construa, acenda e entre num portal do Nether", "flint_and_steel", "dim:1"),
    (0, "story/shiny_gear", "Cubra-me de Diamantes", "Armadura de diamante salva vidas", "diamond_chestplate", "wear:diamond_helmet|diamond_chestplate|diamond_leggings|diamond_boots"),
    (0, "story/enchant_item", "Encantador", "Encante um item numa mesa de encantamento", "enchanted_book", "ev:enchanted"),
    (0, "story/cure_zombie_villager", "Médico Zumbi", "Enfraqueça e cure um aldeão zumbi", "golden_apple", "ev:cured"),
    (0, "story/follow_ender_eye", "Olho Espião", "Siga um olho de ender", "ender_eye", "has:ender_eye"),
    (0, "story/enter_the_end", "O Fim?", "Entre no portal do Fim", "end_stone", "dim:2"),
    // Nether
    (1, "nether/root", "Nether", "Traga roupas de verão", "netherrack", "dim:1"),
    (1, "nether/return_to_sender", "Devolver ao Remetente", "Destrua um ghast com uma bola de fogo", "fire_charge", "ev:ghast_returned"),
    (1, "nether/find_bastion", "Esses Eram os Dias", "Entre num bastião em ruínas", "polished_blackstone_bricks", "ev:bastion"),
    (1, "nether/obtain_ancient_debris", "Escondido nas Profundezas", "Obtenha detritos ancestrais", "ancient_debris", "has:ancient_debris"),
    (1, "nether/fast_travel", "Bolha Subespacial", "Use o Nether para viajar 7 km no Mundo Superior", "map", "ev:fast_travel"),
    (1, "nether/find_fortress", "Uma Fortaleza Terrível", "Entre numa fortaleza do Nether", "nether_bricks", "ev:fortress"),
    (1, "nether/obtain_crying_obsidian", "Quem Está Cortando Cebolas?", "Obtenha obsidiana chorona", "crying_obsidian", "has:crying_obsidian"),
    (1, "nether/distract_piglin", "Ooh, Brilhante", "Distraia piglins com ouro", "gold_ingot", "ev:piglin_gold"),
    (1, "nether/ride_strider", "Este Barco Tem Pernas", "Monte um lavagante com um fungo no palito", "warped_fungus_on_a_stick", "ev:ride_strider"),
    (1, "nether/uneasy_alliance", "Aliança Desconfortável", "Resgate um ghast do Nether e mate-o no Mundo Superior", "ghast_tear", "ev:uneasy_alliance"),
    (1, "nether/loot_bastion", "Guerra dos Porcos", "Saqueie um baú num bastião", "chest", "ev:bastion_loot"),
    (1, "nether/use_lodestone", "País das Pedras-Ímã", "Use uma bússola numa magnetita", "lodestone", "ev:lodestone"),
    (1, "nether/netherite_armor", "Cubra-me de Detritos", "Obtenha uma armadura completa de netherita", "netherite_chestplate", "wearall:netherite_helmet,netherite_chestplate,netherite_leggings,netherite_boots"),
    (1, "nether/get_wither_skull", "Esqueleto Assustador", "Obtenha um crânio de esqueleto wither", "wither_skeleton_skull", "has:wither_skeleton_skull"),
    (1, "nether/obtain_blaze_rod", "Para o Fogo", "Tire uma vara de blaze de um blaze", "blaze_rod", "has:blaze_rod"),
    (1, "nether/charge_respawn_anchor", "Não Exatamente \"Nove\" Vidas", "Carregue uma âncora de renascimento ao máximo", "respawn_anchor", "ev:anchor_full"),
    (1, "nether/explore_nether", "Destinos Quentes", "Explore todos os biomas do Nether", "netherite_boots", "nether_biomes:5"),
    (1, "nether/summon_wither", "Alturas Murchas", "Invoque o Wither", "nether_star", "ev:wither_summoned"),
    (1, "nether/brew_potion", "Cervejaria Local", "Prepare uma poção", "potion_water", "ev:brewed"),
    (1, "nether/create_beacon", "Trazendo o Farol para Casa", "Construa e posicione um sinalizador", "beacon", "ev:beacon:1|beacon:2|beacon:3|beacon:4"),
    (1, "nether/all_potions", "Uma Mistura Furiosa", "Tenha todos os efeitos de poção ao mesmo tempo", "milk_bucket", "effects:12"),
    (1, "nether/create_full_beacon", "Sinalizador", "Leve um sinalizador à potência máxima", "beacon", "ev:beacon:4"),
    (1, "nether/all_effects", "Como Chegamos Aqui?", "Tenha todos os efeitos ao mesmo tempo", "bucket", "effects:24"),
    // The End
    (2, "end/root", "O Fim", "Ou o começo?", "end_stone", "dim:2"),
    (2, "end/kill_dragon", "Liberte o Fim", "Boa sorte", "dragon_head", "ev:killed:ender_dragon|killed:void_guardian"),
    (2, "end/dragon_egg", "A Próxima Geração", "Segure o ovo do dragão", "dragon_egg", "has:dragon_egg"),
    (2, "end/enter_end_gateway", "Fuga Remota", "Escape da ilha", "ender_pearl", "ev:end_gateway"),
    (2, "end/respawn_dragon", "O Fim... De Novo...", "Ressuscite o dragão do Fim", "end_crystal", "ev:dragon_respawned"),
    (2, "end/dragon_breath", "Você Precisa de uma Bala", "Colete bafo do dragão numa garrafa", "dragon_breath", "has:dragon_breath"),
    (2, "end/find_end_city", "A Cidade no Fim do Jogo", "Vá em frente, o que poderia acontecer?", "purpur_block", "ev:end_city"),
    (2, "end/elytra", "O Céu é o Limite", "Encontre élitros", "elytra", "has:elytra"),
    (2, "end/levitate", "Ótima Vista Daqui de Cima", "Levite 50 blocos com os ataques de um shulker", "shulker_shell", "ev:levitate50"),
    // Adventure
    (3, "adventure/root", "Aventura", "Aventura, exploração e combate", "map", "ev:killed_any|stat:mobs_killed>=1"),
    (3, "adventure/voluntary_exile", "Exílio Voluntário", "Mate um capitão de assalto", "white_banner", "ev:killed:marauder"),
    (3, "adventure/spyglass_at_parrot", "É um Pássaro?", "Olhe para um papagaio com uma luneta", "spyglass", "ev:spyglass_bird"),
    (3, "adventure/kill_a_mob", "Caçador de Monstros", "Mate qualquer monstro hostil", "iron_sword", "ev:killed_hostile"),
    (3, "adventure/read_power_of_chiseled_bookshelf", "O Poder dos Livros", "Leia o sinal de uma estante entalhada com um comparador", "chiseled_bookshelf", "ev:read_bookshelf"),
    (3, "adventure/trade", "Que Negócio!", "Faça uma troca com um aldeão", "emerald", "ev:traded|stat:trades>=1"),
    (3, "adventure/trim_with_any_armor_pattern", "Forjando um Novo Visual", "Aplique um acabamento numa armadura na bancada de ferraria", "dune_armor_trim_smithing_template", "ev:trimmed"),
    (3, "adventure/honey_block_slide", "Situação Pegajosa", "Pule num bloco de mel para amortecer a queda", "honey_block", "ev:honey_slide"),
    (3, "adventure/ol_betsy", "Velha Betsy", "Dispare uma besta", "crossbow", "ev:crossbow"),
    (3, "adventure/lightning_rod_with_villager_no_fire", "Protetor contra Surtos", "Proteja um aldeão de um raio sem fogo", "lightning_rod", "ev:surge"),
    (3, "adventure/fall_from_world_height", "Cavernas e Penhascos", "Caia do topo do mundo até o fundo e sobreviva", "water_bucket", "ev:fall_world"),
    (3, "adventure/salvage_sherd", "Respeitando os Restos", "Escove um bloco suspeito e obtenha um caco de cerâmica", "brush", "ev:brushed"),
    (3, "adventure/avoid_vibration", "Furtividade 100", "Agache perto de um sensor sculk ou de um vigia sem ser detectado", "sculk_sensor", "ev:sneak_sensor"),
    (3, "adventure/sleep_in_bed", "Doces Sonhos", "Durma numa cama para mudar seu ponto de renascimento", "red_bed", "ev:slept"),
    (3, "adventure/hero_of_the_village", "Herói da Vila", "Defenda uma vila de um assalto", "white_banner", "ev:raid_won"),
    (3, "adventure/throw_trident", "Uma Piada Sem Graça", "Arremesse um tridente em algo", "trident", "ev:trident_hit"),
    (3, "adventure/kill_mob_near_sculk_catalyst", "Espalhou-se", "Mate um mob perto de um catalisador sculk", "sculk_catalyst", "ev:catalyst_kill"),
    (3, "adventure/shoot_arrow", "Faça Mira", "Acerte algo com uma flecha", "bow", "ev:arrow_hit"),
    (3, "adventure/kill_all_mobs", "Monstros Caçados", "Mate um de cada monstro hostil", "diamond_sword", "hostiles_all"),
    (3, "adventure/totem_of_undying", "Pós-Morte", "Use um totem da imortalidade para enganar a morte", "totem_of_undying", "ev:totem"),
    (3, "adventure/summon_iron_golem", "Ajudante Contratado", "Invoque um golem de ferro", "carved_pumpkin", "ev:golem"),
    (3, "adventure/trade_at_world_height", "Comerciante Estelar", "Troque com um aldeão no limite de altura", "emerald", "ev:trade_high"),
    (3, "adventure/trim_with_all_exclusive_armor_patterns", "Ferraria com Estilo", "Aplique os acabamentos Agulha, Focinho, Costela, Guarda, Silêncio, Vex, Maré e Desbravador", "silence_armor_trim_smithing_template", "trims_rare"),
    (3, "adventure/two_birds_one_arrow", "Dois Coelhos, Uma Flecha", "Mate dois phantoms com uma flecha perfurante", "crossbow", "ev:two_birds"),
    (3, "adventure/whos_the_pillager_now", "Quem é o Saqueador Agora?", "Dê ao saqueador do próprio veneno", "crossbow", "ev:pillager_crossbow"),
    (3, "adventure/arbalistic", "Arbalista", "Mate cinco mobs diferentes com um tiro de besta", "crossbow", "ev:arbalistic"),
    (3, "adventure/adventuring_time", "Hora da Aventura", "Descubra todos os biomas", "diamond_boots", "biomes:15"),
    (3, "adventure/play_jukebox_in_meadows", "Som da Música", "Faça os prados ganharem vida com o som de um toca-discos", "jukebox", "ev:jukebox_meadow"),
    (3, "adventure/walk_on_powder_snow_with_leather_boots", "Leve como um Coelho", "Ande na neve fofa sem afundar", "leather_boots", "ev:powder_walk"),
    (3, "adventure/spyglass_at_ghast", "É um Balão?", "Olhe para um ghast com uma luneta", "spyglass", "ev:spyglass_ghast"),
    (3, "adventure/very_very_frightening", "Muito, Muito Assustador", "Atinja um aldeão com um raio", "trident", "ev:channel_villager"),
    (3, "adventure/sniper_duel", "Duelo de Atiradores", "Mate um esqueleto a pelo menos 50 metros", "arrow", "ev:sniper"),
    (3, "adventure/bullseye", "Na Mosca", "Acerte o centro de um alvo a pelo menos 30 metros", "target", "ev:bullseye"),
    (3, "adventure/minecraft_trials_edition", "Minecraft: Edição Desafios", "Entre numa câmara dos desafios", "chiseled_tuff", "ev:trial_chamber"),
    (3, "adventure/under_lock_and_key", "Sob Chave e Cadeado", "Abra um cofre com uma chave dos desafios", "trial_key", "ev:vault_opened"),
    (3, "adventure/revaulting", "Revoltante", "Abra um cofre agourento com uma chave agourenta", "ominous_trial_key", "ev:ominous_vault"),
    (3, "adventure/blowback", "Contragolpe", "Mate uma brisa com uma carga de vento refletida", "wind_charge", "ev:blowback"),
    (3, "adventure/who_needs_rockets", "Quem Precisa de Foguetes?", "Use uma carga de vento para subir 8 blocos", "wind_charge", "ev:wind_jump"),
    (3, "adventure/crafters_crafting_crafters", "Fabricantes Fabricando Fabricantes", "Esteja perto de um fabricador quando ele fabricar um fabricador", "crafter", "ev:crafter_crafter"),
    (3, "adventure/lighten_up", "Ilumine-se", "Raspe uma lâmpada de cobre com um machado para deixá-la mais clara", "copper_bulb", "ev:bulb_scraped"),
    (3, "adventure/overoverkill", "Exagero do Exagero", "Cause 50 corações de dano num único golpe de maça", "mace", "ev:mace_big"),
    (3, "adventure/heart_transplanter", "Transplante de Coração", "Coloque um coração de rangedor corretamente", "creaking_heart", "ev:creaking_heart"),
    (3, "adventure/recover_compass", "Lembrança Desejada", "Use uma bússola de recuperação para chegar onde morreu", "recovery_compass", "ev:recovered"),
    (3, "adventure/use_spear", "Investida", "Acerte um mob com o avanço de uma lança", "iron_spear", "ev:lunge"),
    // Husbandry
    (4, "husbandry/root", "Criação", "O mundo está cheio de amigos e comida", "hay_block", "stat:play_seconds>=1"),
    (4, "husbandry/safely_harvest_honey", "Abelhas Bem Instaladas", "Use uma fogueira para coletar mel de uma colmeia sem irritar as abelhas", "honey_bottle", "has:honey_bottle"),
    (4, "husbandry/breed_an_animal", "Os Pais e as Mães", "Reproduza dois animais", "wheat", "ev:bred"),
    (4, "husbandry/tame_an_animal", "Melhores Amigos para Sempre", "Dome um animal", "bone", "ev:tamed"),
    (4, "husbandry/fishy_business", "Negócio de Peixes", "Pesque um peixe", "fishing_rod", "has:cod|salmon|tropical_fish|pufferfish|raw_fish"),
    (4, "husbandry/plant_seed", "Um Lugar Semeado", "Plante uma semente e veja-a crescer", "wheat_seeds", "ev:planted"),
    (4, "husbandry/tactical_fishing", "Pesca Tática", "Pegue um peixe... sem vara de pesca!", "water_bucket", "has:cod_bucket|salmon_bucket|pufferfish_bucket|tropical_fish_bucket"),
    (4, "husbandry/balanced_diet", "Dieta Balanceada", "Coma tudo que é comestível", "apple", "stat:foods_eaten>=40"),
    (4, "husbandry/obtain_netherite_hoe", "Dedicação Séria", "Use um lingote de netherita para melhorar uma enxada", "netherite_hoe", "has:netherite_hoe"),
    (4, "husbandry/silk_touch_nest", "Pequena Mudança", "Mova uma colmeia com 3 abelhas usando toque suave", "bee_nest", "ev:silk_nest"),
    (4, "husbandry/wax_on", "Encerar", "Aplique favo de mel num bloco de cobre", "honeycomb", "ev:waxed"),
    (4, "husbandry/wax_off", "Desencerar", "Raspe a cera de um bloco de cobre", "stone_axe", "ev:unwaxed"),
    (4, "husbandry/axolotl_in_a_bucket", "O Predador Mais Fofo", "Pegue um axolote num balde", "axolotl_bucket", "has:axolotl_bucket"),
    (4, "husbandry/tadpole_in_a_bucket", "Bem-Vindo ao Balde", "Pegue um girino num balde", "tadpole_bucket", "has:tadpole_bucket"),
    (4, "husbandry/make_a_sign_glow", "Brilho e Maravilha", "Faça o texto de uma placa brilhar", "glow_ink_sac", "ev:glow_sign"),
    (4, "husbandry/ride_a_boat_with_a_goat", "Qualquer Barco Flutua", "Entre num barco e flutue com uma cabra", "oak_boat", "ev:goat_boat"),
    (4, "husbandry/allay_deliver_item_to_player", "Você Tem um Amigo em Mim", "Receba itens de um allay", "cookie", "ev:allay"),
    (4, "husbandry/feed_snifflet", "Pequenas Fungadas", "Alimente um farejadorzinho", "torchflower_seeds", "ev:fed_snifflet"),
    (4, "husbandry/obtain_sniffer_egg", "Cheiro Interessante", "Obtenha um ovo de farejador", "sniffer_egg", "has:sniffer_egg"),
    (4, "husbandry/plant_any_sniffer_seed", "Plantando o Passado", "Plante qualquer semente de farejador", "pitcher_pod", "ev:planted_ancient"),
    (4, "husbandry/froglights", "Com Nossos Poderes Combinados!", "Tenha todas as luzes de sapo no inventário", "verdant_froglight", "allhas:ochre_froglight,pearlescent_froglight,verdant_froglight"),
    (4, "husbandry/leash_all_frog_variants", "Quando o Bando Pula na Cidade", "Prenda cada variante de sapo numa guia", "lead", "ev:frogs_leashed"),
    (4, "husbandry/bred_all_animals", "Dois a Dois", "Reproduza todos os animais", "golden_carrot", "stat:breeds>=12"),
    (4, "husbandry/complete_catalogue", "Um Catálogo Completo de Gatos", "Dome todas as variantes de gato", "cod", "ev:cats_all"),
    (4, "husbandry/whole_pack", "A Matilha Completa", "Dome todas as variantes de lobo", "bone", "ev:wolves_all"),
    (4, "husbandry/repair_wolf_armor", "Bom como Novo", "Conserte uma armadura de lobo danificada com escamas de tatu", "armadillo_scute", "ev:wolf_armor"),
    (4, "husbandry/remove_wolf_armor", "Corte Tosado", "Remova a armadura de um lobo com uma tesoura", "shears", "ev:wolf_armor_off"),
    (4, "husbandry/place_dried_ghast_in_water", "Fique Hidratado!", "Coloque um ghast seco na água", "dried_ghast", "ev:dried_ghast"),
    (4, "husbandry/kill_axolotl_target", "O Poder Curativo da Amizade!", "Junte-se a um axolote e vença uma luta", "tropical_fish_bucket", "ev:axolotl_help"),
];

/// Hostile mobs the "kill one of each" advancement asks for (the ones this world has).
fn hostiles(game: &crate::Game) -> Vec<String> {
    game.mobs.reg.defs.iter().filter(|d| d.kind == voxel::mob::Kind::Hostile && !d.boss).map(|d| d.name.clone()).collect()
}

/// Rare trim patterns (the "Smithing with Style" set).
const RARE_TRIMS: [&str; 8] = ["spire", "snout", "rib", "ward", "silence", "vex", "tide", "wayfinder"];

/// The tokens an event makes (an event can satisfy several rules).
fn tokens(e: &Event, game: &crate::Game, out: &mut HashSet<String>) {
    match e {
        Event::Enchanted => {
            out.insert("enchanted".into());
        }
        Event::Brewed => {
            out.insert("brewed".into());
        }
        Event::Drank(p) => {
            out.insert(format!("drank:{p}"));
        }
        Event::Smithed(i) => {
            out.insert(format!("smithed:{i}"));
        }
        Event::Trimmed => {
            out.insert("trimmed".into());
        }
        Event::Killed(m) => {
            out.insert(format!("killed:{m}"));
            out.insert("killed_any".into());
            if game.mobs.reg.id(m).map(|i| game.mobs.reg.defs[i].kind == voxel::mob::Kind::Hostile).unwrap_or(false) {
                out.insert("killed_hostile".into());
            }
        }
        Event::Brushed => {
            out.insert("brushed".into());
        }
        Event::Shrieked => {
            out.insert("shrieked".into());
        }
        Event::WardenSpawned => {
            out.insert("warden".into());
        }
        Event::VaultOpened => {
            out.insert("vault_opened".into());
        }
        Event::SpawnerBeaten => {
            out.insert("spawner_beaten".into());
        }
        Event::BeaconOn(n) => {
            out.insert(format!("beacon:{n}"));
        }
        Event::ConduitOn => {
            out.insert("conduit".into());
        }
        Event::MaceSmash(d) => {
            out.insert("mace".into());
            if *d >= 100.0 {
                out.insert("mace_big".into());
            }
        }
        Event::Recovered => {
            out.insert("recovered".into());
        }
        Event::Sniffed => {
            out.insert("sniffed".into());
        }
        Event::Repaired => {
            out.insert("repaired".into());
        }
        Event::Ground => {
            out.insert("ground".into());
        }
        Event::Token(t) => {
            out.insert(t.clone());
        }
    }
}

impl crate::Game {
    fn holds(&self, name: &str) -> bool {
        let Some(id) = self.items.id(name) else { return false };
        self.surv.inv.count_of(id) > 0
            || self.surv.armor.iter().flatten().any(|s| s.item == id)
            || self.surv.offhand.map(|s| s.item == id).unwrap_or(false)
            || self.surv.cursor.map(|s| s.item == id).unwrap_or(false)
    }

    fn wears(&self, name: &str) -> bool {
        let Some(id) = self.items.id(name) else { return false };
        self.surv.armor.iter().flatten().any(|s| s.item == id)
    }

    /// Is the rule met now (events this tick in `ev`)?
    fn rule_met(&self, rule: &str, ev: &HashSet<String>) -> bool {
        let (kind, arg) = rule.split_once(':').unwrap_or((rule, ""));
        match kind {
            "has" => arg.split('|').any(|n| self.holds(n)),
            "allhas" => arg.split(',').all(|n| self.holds(n)),
            "wear" => arg.split('|').any(|n| self.wears(n)),
            "wearall" => arg.split(',').all(|n| self.wears(n)),
            "ev" => {
                // "ev:a|b|stat:x>=1" mixes events with a stat fallback.
                arg.split('|').any(|t| ev.contains(t)) || rule.contains("|stat:") && self.rule_met(&rule[rule.find("stat:").unwrap()..], ev)
            }
            "stat" => {
                let (k, n) = arg.split_once(">=").unwrap_or((arg, "1"));
                self.prog.stats.get(k) >= n.parse().unwrap_or(1)
            }
            "dim" => self.dim.as_u8().to_string() == arg || self.prog.stats.get(&format!("dim:{arg}")) > 0,
            "biomes" => (0..15).filter(|b| self.prog.stats.get(&format!("biome:{b}")) > 0).count() >= arg.parse().unwrap_or(15),
            "nether_biomes" => [15, 16, 17, 19].iter().filter(|b| self.prog.stats.get(&format!("biome:{b}")) > 0).count() >= 4,
            "lvl" => self.surv.level().0 >= arg.parse().unwrap_or(30),
            "effects" => self.prog.effects.len() >= arg.parse().unwrap_or(12),
            "hostiles_all" => {
                let h = hostiles(self);
                !h.is_empty() && h.iter().all(|m| self.prog.stats.get(&format!("killed:{m}")) > 0)
            }
            "trims_rare" => RARE_TRIMS.iter().all(|t| self.prog.stats.get(&format!("trim:{t}")) > 0),
            _ => false,
        }
    }

    pub(crate) fn update_advancements(&mut self, dt: f32) {
        // Where the player is: dimension and biome, for the exploring advancements.
        let d = self.dim.as_u8();
        self.prog.stats.counts.entry(format!("dim:{d}")).or_insert(1);
        let p = self.body.pos;
        if self.world_open {
            let b = self.world.gen.biome_at(p.x.floor() as i32, p.z.floor() as i32) as u8;
            let key = format!("biome:{b}");
            if self.prog.stats.get(&key) == 0 {
                self.prog.stats.add(&key, 1);
            }
        }
        self.prog.last_pos = p;
        // Recovery compass: reaching the death spot with it in hand.
        if let Some((dp, dd)) = self.prog.death {
            if dd == d && (dp - p).length() < 4.0 && self.held_key() == "recovery_compass" {
                self.prog.event(Event::Recovered);
                self.prog.death = None;
            }
        }
        let events = std::mem::take(&mut self.prog.events);
        let mut ev = HashSet::new();
        for e in &events {
            tokens(e, self, &mut ev);
            if let Event::Smithed(t) = e {
                if let Some(p) = t.strip_prefix("trim:") {
                    self.prog.stats.add(&format!("trim:{p}"), 1);
                }
            }
        }
        self.prog.adv_t += dt;
        if ev.is_empty() && self.prog.adv_t < 1.0 {
            return;
        }
        self.prog.adv_t = 0.0;
        if !self.survival && ev.is_empty() {
            // Creative players earn by doing, not by holding everything.
            return;
        }
        for a in ADVANCEMENTS {
            if self.prog.earned.iter().any(|k| k == a.1) {
                continue;
            }
            if self.rule_met(a.5, &ev) {
                self.prog.earned.push(a.1.to_string());
                self.prog.pending_toasts.push(a.1.to_string());
                self.sfx(crate::audio::Sfx::Xp);
            }
        }
    }

    /// The top-right card for a fresh advancement (four seconds each).
    pub(crate) fn draw_advancement_toast(&mut self, w: f32) {
        let Some(key) = self.prog.pending_toasts.first().cloned() else { return };
        if self.prog.toast_t == 0.0 {
            self.prog.toast_t = self.now;
        }
        let age = self.now - self.prog.toast_t;
        if age > 4.0 {
            self.prog.pending_toasts.remove(0);
            self.prog.toast_t = 0.0;
            return;
        }
        let Some(a) = ADVANCEMENTS.iter().find(|a| a.1 == key) else {
            self.prog.pending_toasts.remove(0);
            return;
        };
        let s = self.ui.scale;
        let slide = ((age * 4.0).min(1.0) * ((4.0 - age) * 4.0).min(1.0)).clamp(0.0, 1.0);
        let (tw, th) = (160.0 * s, 32.0 * s);
        let x = w - tw * slide;
        let y = 2.0 * s + 30.0 * s * (!self.prog.effects.is_empty()) as u8 as f32;
        self.ui.rect(x, y, tw, th, [33, 33, 33, 235]);
        self.ui.frame(x, y, tw, th, s, [120, 80, 160, 255]);
        self.draw_item_name_icon(a.4, x + 8.0 * s, y + 8.0 * s, 16.0 * s);
        let head = if a.1.ends_with("root") { "Nova aba!" } else { "Progresso alcançado!" };
        self.ui.text_scaled(x + 30.0 * s, y + 6.0 * s, head, [255, 255, 85, 255], s);
        self.ui.text_scaled(x + 30.0 * s, y + 18.0 * s, a.2, [255, 255, 255, 255], s);
    }

    /// An item's icon by name (nothing when the item doesn't exist in this world).
    fn draw_item_name_icon(&mut self, name: &str, x: f32, y: f32, size: f32) {
        if let Some(id) = self.items.id(name) {
            crate::survival::item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &ItemStack::new(id, 1), x, y, size);
        } else {
            self.ui.rect(x + size * 0.25, y + size * 0.25, size * 0.5, size * 0.5, [120, 80, 160, 255]);
        }
    }

    /// The locator bar: a strip at the top of the screen that shows, by direction, where the
    /// last death point, the spawn point and lit beacons are. Points fade with distance and
    /// drift up or down when they are above or below.
    pub(crate) fn draw_locator_bar(&mut self, w: f32, _hx: f32, _hy: f32) {
        if self.hide_hud || self.screen != crate::Screen::Playing {
            return;
        }
        let mut points: Vec<(Vec3, [u8; 4])> = Vec::new();
        let d = self.dim.as_u8();
        if let Some((p, dd)) = self.prog.death {
            if dd == d {
                points.push((p, [200, 60, 60, 255]));
            }
        }
        if d == 0 {
            if let Some(sp) = self.surv.spawn {
                points.push((sp.as_vec3() + Vec3::splat(0.5), [235, 235, 235, 255]));
            }
        }
        for (pos, b) in &self.prog.beacons {
            if b.levels > 0 && d == 0 {
                points.push((pos.as_vec3() + Vec3::splat(0.5), [120, 220, 255, 255]));
            }
        }
        if points.is_empty() {
            return;
        }
        let s = self.ui.scale;
        let bw = 182.0 * s;
        let x0 = (w - bw) / 2.0;
        let y0 = 3.0 * s;
        self.ui.rect(x0, y0 + 2.0 * s, bw, 1.0 * s, [0, 0, 0, 120]);
        self.ui.rect(x0, y0 + 2.0 * s, s, 4.0 * s, [0, 0, 0, 120]);
        self.ui.rect(x0 + bw - s, y0 + 2.0 * s, s, 4.0 * s, [0, 0, 0, 120]);
        let eye = self.body.eye();
        let fwd = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        for (p, c) in points {
            let to = p - eye;
            let flat = Vec3::new(to.x, 0.0, to.z);
            let dist = flat.length();
            if dist < 1.0 {
                continue;
            }
            let ang = flat.dot(right).atan2(flat.dot(fwd));
            // 60° either side fills the bar; beyond that the point waits at the edge.
            let k = (ang / std::f32::consts::FRAC_PI_3).clamp(-1.0, 1.0);
            let x = x0 + bw / 2.0 + k * (bw / 2.0 - 4.0 * s);
            let size = if dist < 64.0 { 5.0 } else if dist < 512.0 { 4.0 } else { 3.0 } * s;
            let dy = if to.y > 6.0 { -2.0 * s } else if to.y < -6.0 { 2.0 * s } else { 0.0 };
            let edge = k.abs() >= 1.0;
            let mut col = c;
            if edge {
                col[3] = 140;
            }
            self.ui.rect(x - size / 2.0 - s, y0 + 2.5 * s - size / 2.0 + dy - s, size + 2.0 * s, size + 2.0 * s, [0, 0, 0, 160]);
            self.ui.rect(x - size / 2.0, y0 + 2.5 * s - size / 2.0 + dy, size, size, col);
        }
        // The recovery compass in hand: a needle under the crosshair toward the death spot.
        if self.held_key() == "recovery_compass" {
            let h = self.window.inner_size().height as f32;
            let (cx, cy) = (w / 2.0, h / 2.0 + 26.0 * s);
            let target = self.prog.death.filter(|(_, dd)| *dd == d).map(|(p, _)| p);
            let ang = match target {
                Some(p) => {
                    let flat = Vec3::new(p.x - eye.x, 0.0, p.z - eye.z);
                    flat.dot(right).atan2(flat.dot(fwd))
                }
                // Nowhere to point: it spins.
                None => self.now * 7.0,
            };
            self.ui.rect(cx - 6.0 * s, cy - 6.0 * s, 12.0 * s, 12.0 * s, [20, 40, 50, 160]);
            for i in 0..5 {
                let t = i as f32 * 1.2 * s;
                let (px, py) = (cx + ang.sin() * t, cy - ang.cos() * t);
                self.ui.rect(px - s * 0.5, py - s * 0.5, s, s, [60, 230, 210, 255]);
            }
        }
    }

    /// The advancements screen (L): five tabs of advancements and a statistics page.
    pub(crate) fn draw_advancements(&mut self, w: f32, h: f32) {
        if !self.prog.show_advancements {
            return;
        }
        let s = self.ui.scale;
        let (pw, ph) = ((300.0 * s).min(w - 8.0 * s), (220.0 * s).min(h - 8.0 * s));
        let (x0, y0) = ((w - pw) / 2.0, (h - ph) / 2.0);
        self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 120]);
        self.ui.rect(x0, y0, pw, ph, [26, 26, 30, 240]);
        self.ui.frame(x0, y0, pw, ph, s, [160, 160, 160, 255]);
        // Tabs: five advancement tabs and Statistics.
        let tab_w = pw / 6.0;
        for t in 0..6 {
            let on = self.prog.adv_tab == t;
            let tx = x0 + t as f32 * tab_w;
            self.ui.rect(tx + s, y0 + s, tab_w - 2.0 * s, 12.0 * s, if on { [70, 70, 90, 255] } else { [40, 40, 46, 255] });
            let label = if t < 5 { TABS[t] } else { "Estatísticas" };
            let lw = self.ui.text_width(label, s * 0.75);
            self.ui.text_scaled(tx + tab_w / 2.0 - lw / 2.0, y0 + 4.0 * s, label, [230, 230, 230, 255], s * 0.75);
        }
        let earned_total = self.prog.earned.len();
        let foot = format!("{earned_total}/{} progressos   ←/→ abas   ↑/↓ rolar   L fechar", ADVANCEMENTS.len());
        self.ui.text_scaled(x0 + 4.0 * s, y0 + ph - 9.0 * s, &foot, [170, 170, 170, 255], s * 0.75);
        let rows_y = y0 + 16.0 * s;
        let row_h = 18.0 * s;
        let fit = ((ph - 28.0 * s) / row_h) as usize;
        if self.prog.adv_tab == 5 {
            let mut lines: Vec<(String, String)> = Vec::new();
            let st = &self.prog.stats;
            let km = |cm: u64| format!("{:.2} km", cm as f32 / 100_000.0);
            lines.push(("Tempo jogado".into(), crate::effects::clock(st.get("play_seconds") as f32)));
            lines.push(("Distância a pé".into(), km(st.get("cm_walked"))));
            lines.push(("Distância de carrinho".into(), km(st.get("cm_by_cart"))));
            lines.push(("Distância nadando".into(), km(st.get("cm_swum"))));
            lines.push(("Distância voando".into(), km(st.get("cm_flown"))));
            lines.push(("Pulos".into(), st.get("jumps").to_string()));
            lines.push(("Blocos minerados".into(), st.get("blocks_mined").to_string()));
            lines.push(("Mobs mortos".into(), st.get("mobs_killed").to_string()));
            lines.push(("Mortes".into(), st.get("deaths").to_string()));
            lines.push(("Itens encantados".into(), st.get("items_enchanted").to_string()));
            lines.push(("Poções preparadas".into(), st.get("potions_brewed").to_string()));
            lines.push(("Dano causado".into(), format!("{:.1}", st.get("damage_dealt") as f32 / 10.0)));
            lines.push(("Dano recebido".into(), format!("{:.1}", st.get("damage_taken") as f32 / 10.0)));
            lines.push(("Blocos escovados".into(), st.get("brushed").to_string()));
            lines.push(("Cofres abertos".into(), st.get("vaults").to_string()));
            let mut kills: Vec<(String, u64)> = st.counts.iter().filter_map(|(k, v)| k.strip_prefix("killed:").map(|m| (m.to_string(), *v))).collect();
            kills.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            for (m, n) in kills {
                let dn = self.mobs.reg.id(&m).map(|i| self.mobs.reg.defs[i].display_name.clone()).unwrap_or(m);
                lines.push((format!("Matou: {dn}"), n.to_string()));
            }
            let fit = fit * 2;
            self.prog.adv_scroll = self.prog.adv_scroll.min(lines.len().saturating_sub(fit));
            for (i, (k, v)) in lines.iter().skip(self.prog.adv_scroll).take(fit).enumerate() {
                let y = rows_y + i as f32 * 9.0 * s;
                self.ui.text_scaled(x0 + 8.0 * s, y, k, [220, 220, 220, 255], s * 0.8);
                let vw = self.ui.text_width(v, s * 0.8);
                self.ui.text_scaled(x0 + pw - 8.0 * s - vw, y, v, [255, 255, 160, 255], s * 0.8);
            }
            return;
        }
        let list: Vec<&Adv> = ADVANCEMENTS.iter().filter(|a| a.0 as usize == self.prog.adv_tab).collect();
        self.prog.adv_scroll = self.prog.adv_scroll.min(list.len().saturating_sub(fit));
        for (i, a) in list.iter().skip(self.prog.adv_scroll).take(fit).enumerate() {
            let y = rows_y + i as f32 * row_h;
            let done = self.prog.earned.iter().any(|k| k == a.1);
            self.ui.rect(x0 + 4.0 * s, y, pw - 8.0 * s, row_h - 2.0 * s, if done { [60, 52, 20, 255] } else { [38, 38, 44, 255] });
            self.ui.frame(x0 + 4.0 * s, y, 18.0 * s, row_h - 2.0 * s, s, if done { [230, 190, 60, 255] } else { [90, 90, 90, 255] });
            self.draw_item_name_icon(a.4, x0 + 5.0 * s, y + 0.5 * s, 15.0 * s);
            let col = if done { [255, 255, 120, 255] } else { [220, 220, 220, 255] };
            self.ui.text_scaled(x0 + 26.0 * s, y + 1.5 * s, a.2, col, s * 0.85);
            self.ui.text_scaled(x0 + 26.0 * s, y + 9.0 * s, a.3, [150, 150, 150, 255], s * 0.7);
        }
    }

    /// Keys while the advancements screen is up. True when the key was taken.
    pub(crate) fn advancements_key(&mut self, code: winit::keyboard::KeyCode) -> bool {
        use winit::keyboard::KeyCode;
        if self.screen != crate::Screen::Playing {
            return false;
        }
        if code == KeyCode::KeyL {
            self.prog.show_advancements = !self.prog.show_advancements;
            self.prog.adv_scroll = 0;
            return true;
        }
        if !self.prog.show_advancements {
            return false;
        }
        match code {
            KeyCode::ArrowLeft => self.prog.adv_tab = (self.prog.adv_tab + 5) % 6,
            KeyCode::ArrowRight | KeyCode::Tab => self.prog.adv_tab = (self.prog.adv_tab + 1) % 6,
            KeyCode::ArrowUp => self.prog.adv_scroll = self.prog.adv_scroll.saturating_sub(1),
            KeyCode::ArrowDown => self.prog.adv_scroll += 1,
            KeyCode::Escape => self.prog.show_advancements = false,
            _ => return false,
        }
        if matches!(code, KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Tab) {
            self.prog.adv_scroll = 0;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unique_and_tabs_have_roots() {
        let mut seen = HashSet::new();
        for a in ADVANCEMENTS {
            assert!(seen.insert(a.1), "duplicate {}", a.1);
            assert!((a.0 as usize) < TABS.len());
        }
        for t in 0..TABS.len() {
            assert!(ADVANCEMENTS.iter().any(|a| a.0 as usize == t && a.1.ends_with("root")));
        }
        assert!(ADVANCEMENTS.len() >= 120);
    }
}

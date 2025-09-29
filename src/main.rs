use dotenv::dotenv;
use std::env;
use frankenstein::client_reqwest::Bot;
use frankenstein::methods::{GetUpdatesParams, SendMessageParams, SendPhotoParams};
use frankenstein::updates::UpdateContent;
use frankenstein::types::Message;
use frankenstein::input_file::InputFile;
use frankenstein::AsyncTelegramApi;
use tokio::time::{sleep, Duration};
use gemini_rust::{Gemini, Part};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use chrono::{NaiveDate, Local};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Tickets {
    tickets: String,
    vip_tickets: String,
    selected_ticket: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Papel {
    name: String,
    emoji: String,
    nicks: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Claim {
    role_name: String,
    role_emoji: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Player {
    name: String,
    user: String,
    points: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Game {
    date: String,
    time: String,
    day_of_week: String,
    teams: Vec<String>,
    phase: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Mission {
    title: String,
    text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CrewMember {
    username: String,
    first_name: String,
    is_crewmember: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Crew {
    captain: Vec<CrewMember>,
    leader: Vec<CrewMember>,
    #[serde(rename = "sub-leader")]
    sub_leader: Vec<CrewMember>,
    crew: Vec<CrewMember>,
    subs: Vec<CrewMember>,
}

fn read_tickets() -> Result<HashMap<String, Tickets>, String> {
    let data = fs::read_to_string("tickets.json").map_err(|e| e.to_string())?;
    let tickets: HashMap<String, Tickets> = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(tickets)
}

fn read_receitas() -> Result<HashMap<String, String>, String> {
    let data = fs::read_to_string("receitas.json").map_err(|e| e.to_string())?;
    let receitas: HashMap<String, String> = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(receitas)
}

fn read_pecas() -> Result<HashMap<String, String>, String> {
    let data = fs::read_to_string("pecas.json").map_err(|e| e.to_string())?;
    let pecas: HashMap<String, String> = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(pecas)
}

fn read_papeis() -> Result<Vec<Papel>, String> {
    let data = fs::read_to_string("papeis.json").map_err(|e| e.to_string())?;
    let papeis: Vec<Papel> = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(papeis)
}

fn read_claims() -> Result<HashMap<String, Claim>, String> {
    let data = fs::read_to_string("claims.json").map_err(|e| e.to_string())?;
    let claims: HashMap<String, Claim> = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(claims)
}

fn write_claims(claims: &HashMap<String, Claim>) -> Result<(), String> {
    let data = serde_json::to_string_pretty(claims).map_err(|e| e.to_string())?;
    fs::write("claims.json", data).map_err(|e| e.to_string())?;
    Ok(())
}

fn read_team(team_name: &str) -> Result<Vec<Player>, String> {
    let file_path = format!("{}.json", team_name);
    let data = fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    let players: Vec<Player> = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(players)
}

fn read_calendar() -> Result<Vec<Game>, String> {
    let data = fs::read_to_string("calendario.json").map_err(|e| e.to_string())?;
    let games: Vec<Game> = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(games)
}

fn read_missions() -> Result<Mission, String> {
    let data = fs::read_to_string("missoes.json").map_err(|e| e.to_string())?;
    let mission: Mission = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(mission)
}

fn read_crew() -> Result<Crew, String> {
    let data = fs::read_to_string("tripulantes.json").map_err(|e| e.to_string())?;
    let crew: Crew = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(crew)
}

fn write_crew(crew: &Crew) -> Result<(), String> {
    let data = serde_json::to_string_pretty(crew).map_err(|e| e.to_string())?;
    fs::write("tripulantes.json", data).map_err(|e| e.to_string())?;
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let bot = Bot::new(&token);

    let mut update_params = GetUpdatesParams::builder().build();

    println!("Bot is running...");

    loop {
        let result = bot.get_updates(&update_params).await;

        match result {
            Ok(response) => {
                for update in response.result {
                    if let UpdateContent::Message(message) = update.content {
                        let bot_clone = bot.clone();
                        tokio::spawn(async move {
                            process_message(*message, bot_clone).await;
                        });
                    }
                    update_params.offset = Some((update.update_id + 1) as i64);
                }
            }
            Err(error) => {
                println!("Failed to get updates: {:?}", error);
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn process_message(message: Message, bot: Bot) {
    if let Some(text) = &message.text {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open("chat_log.txt")
            .unwrap();

        if let Some(user) = &message.from {
            let log = format!("[{}] {}: {}\n", Local::now().to_rfc2822(), user.first_name, text);
            if let Err(e) = writeln!(file, "{}", log) {
                eprintln!("Couldn't write to file: {}", e);
            }
        }

        if text == "/bemvindos" {
            let send_photo_params = SendPhotoParams::builder()
                .chat_id(message.chat.id)
                .photo(frankenstein::input_file::FileUpload::InputFile(InputFile { path: "holandesvoador.jpg".into() }))
                .caption("Bem-vindos ao Holandês Voador.")
                .build();

            if let Err(err) = bot.send_photo(&send_photo_params).await {
                println!("Failed to send photo: {:?}", err);
            }

            let response_part1 = "Homens e mulheres do mar... escutem bem.\n\n\
                            Vocês deixaram para trás a vida que conheciam. O tempo, para vocês, não passará da mesma forma que lá fora. A bordo deste navio, não há velhice — mas há serviço. E honra.\n\n\
                            Sejam bem-vindos ao Holandês Voador.\n\
                            Navegaremos por águas que nenhum outro navio ousa cruzar. Levaremos as almas dos que se afogam, dos que se perdem, dos que clamam por redenção. Nosso dever é eterno — mas não sem propósito.";

            let send_message_params1 = SendMessageParams::builder()
                .chat_id(message.chat.id)
                .text(response_part1)
                .build();
            if let Err(err) = bot.send_message(&send_message_params1).await {
                println!("Failed to send message: {:?}", err);
            }

            let response_part2 = "Alguns de vocês vieram por escolha. Outros... por necessidade. Mas todos aqui têm a segunda chance. E comigo no leme, não haverá açoite, nem traição, nem pactos quebrados. O Holandês já conheceu mentiras demais sob seu casco.\n\n\
                            Vocês me servirão, e eu servirei a vocês.\n\
                            Cada nó atado, cada vela içada, cada sino soado nesta embarcação carrega o peso de algo maior: a travessia entre mundos. Se honrarem esse navio e seus deveres, serão lembrados — mesmo nas águas mais escuras da lenda.\n\n\
                            Então preparem-se, tripulação.\n\
                            O mar nos chama, e o tempo já não nos pertence. Que os ventos soprem a nosso favor...\n\
                            ...e que jamais esqueçam:\n\
                            Aqui, sob a minha bandeira, a morte não é o fim — é apenas o começo.";

            let send_message_params2 = SendMessageParams::builder()
                .chat_id(message.chat.id)
                .text(response_part2)
                .build();
            if let Err(err) = bot.send_message(&send_message_params2).await {
                println!("Failed to send message: {:?}", err);
            }
        } else if text.starts_with("/will") {
            let question = text.trim_start_matches("/will").trim();
            if question.is_empty() {
                let send_message_params = SendMessageParams::builder()
                    .chat_id(message.chat.id)
                    .text("Por favor, forneça uma pergunta após o comando /will.")
                    .build();
                if let Err(err) = bot.send_message(&send_message_params).await {
                    println!("Failed to send message: {:?}", err);
                }
                return;
            }
            match ask_gemini(question).await {
                Ok(response) => {
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(response)
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
                Err(err) => {
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(format!("Error asking Gemini: {}", err))
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
            }
        } else if text == "/calendario" {
            match read_calendar() {
                Ok(games) => {
                    let my_team_games = games.into_iter().filter(|game| game.teams.contains(&"🫀".to_string())).collect::<Vec<Game>>();
                    let mut response = "🗓 Calendário de Jogos do seu time:\n\n".to_string();
                    for game in my_team_games {
                        response.push_str(&format!("{} - {} às {} ({}) - {}\n", game.date, game.day_of_week, game.time, game.phase, game.teams.join(" vs ")));
                    }
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(response)
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
                Err(err) => {
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(format!("Erro ao ler o calendário: {}", err))
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
            }
        } else if text == "/proximojogo" {
            match read_calendar() {
                Ok(games) => {
                    let my_team_games = games.into_iter().filter(|game| game.teams.contains(&"🫀".to_string())).collect::<Vec<Game>>();
                    let mut next_game: Option<Game> = None;
                    let today = Local::now().date_naive();

                    for game in my_team_games {
                        let game_date = NaiveDate::parse_from_str(&format!("{}/2025", game.date), "%d/%m/%Y").unwrap();
                        if game_date >= today {
                            if let Some(next) = &next_game {
                                let next_date = NaiveDate::parse_from_str(&format!("{}/2025", next.date), "%d/%m/%Y").unwrap();
                                if game_date < next_date {
                                    next_game = Some(game);
                                }
                            } else {
                                next_game = Some(game);
                            }
                        }
                    }

                    if let Some(game) = next_game {
                        let response = format!("Próximo Jogo:\n\n{} - {} às {} ({}) - {}", game.date, game.day_of_week, game.time, game.phase, game.teams.join(" vs "));
                        let send_message_params = SendMessageParams::builder()
                            .chat_id(message.chat.id)
                            .text(response)
                            .build();
                        if let Err(err) = bot.send_message(&send_message_params).await {
                            println!("Failed to send message: {:?}", err);
                        }
                    } else {
                        let send_message_params = SendMessageParams::builder()
                            .chat_id(message.chat.id)
                            .text("Não há próximos jogos para o seu time.".to_string())
                            .build();
                        if let Err(err) = bot.send_message(&send_message_params).await {
                            println!("Failed to send message: {:?}", err);
                        }
                    }
                }
                Err(err) => {
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(format!("Erro ao ler o calendário: {}", err))
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
            }
        } else if text == "/calendariocompleto" {
            match read_calendar() {
                Ok(games) => {
                    let mut response = "🗓 Calendário de Jogos Completo:\n\n".to_string();
                    for game in games {
                        response.push_str(&format!("{} - {} às {} ({}) - {}\n", game.date, game.day_of_week, game.time, game.phase, game.teams.join(" vs ")));
                    }
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(response)
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
                Err(err) => {
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(format!("Erro ao ler o calendário: {}", err))
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
            }
        } else if text == "/missoes" {
            match read_missions() {
                Ok(mission) => {
                    let response = format!("{}\n\n{}", mission.title, mission.text);
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(response)
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
                Err(err) => {
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(format!("Erro ao ler as missões: {}", err))
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
            }
        } else if text == "/tripulacao" {
            match read_crew() {
                Ok(crew) => {
                    let mut response = "Tripulação do Holandês Voador:\n\n".to_string();
                    response.push_str("Capitão:\n");
                    for member in crew.captain {
                        response.push_str(&format!("- {} (@{})\n", member.first_name, member.username));
                    }
                    response.push_str("\nLíder:\n");
                    for member in crew.leader {
                        response.push_str(&format!("- {} (@{})\n", member.first_name, member.username));
                    }
                    response.push_str("\nSub-Líder:\n");
                    for member in crew.sub_leader {
                        response.push_str(&format!("- {} (@{})\n", member.first_name, member.username));
                    }
                    response.push_str("\nTripulantes:\n");
                    for member in crew.crew {
                        response.push_str(&format!("- {} (@{})\n", member.first_name, member.username));
                    }
                    response.push_str("\nSubs:\n");
                    for member in crew.subs {
                        response.push_str(&format!("- {} (@{})\n", member.first_name, member.username));
                    }

                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(response)
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
                Err(err) => {
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(format!("Erro ao ler a lista de tripulantes: {}", err))
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
            }
        } else if text == "/comandos" {
            let response = "Comandos disponíveis:\n\n\
/will [pergunta] - Faça uma pergunta para o Will Turner.\n\
/calendario - Mostra o calendário de jogos do seu time.\n\
/proximojogo - Mostra o próximo jogo do seu time.\n\
/calendariocompleto - Mostra o calendário de jogos completo.\n\
/missoes - Mostra a pontuação das missões.\n\
/tripulacao - Lista a tripulação do Holandês Voador.\n\
/tickets {nome} - Mostra os tickets de um jogador.\n\
/receitas {nome} - Mostra as receitas de um jogador.\n\
/pecas {nome} - Mostra as peças de um jogador.\n\
/claim {nick} - Reivindica um papel.\n\
/claims - Mostra a lista de papéis reivindicados.\n\
/reset - Limpa a lista de papéis reivindicados.\n\
/barbossa - Mostra a pontuação do time Barbossa.\n\
/jack - Mostra a pontuação do time Jack Sparrow.\n\
/elizabeth - Mostra a pontuação do time Elizabeth Swann.\n\
/bemvindos - Envia a mensagem de boas vindas com a foto do Holandês Voador.";
            let send_message_params = SendMessageParams::builder()
                .chat_id(message.chat.id)
                .text(response)
                .build();
            if let Err(err) = bot.send_message(&send_message_params).await {
                println!("Failed to send message: {:?}", err);
            }
        } else if text.starts_with("/tickets") {
            let args: Vec<&str> = text.split_whitespace().collect();
            if args.len() < 2 {
                let send_message_params = SendMessageParams::builder()
                    .chat_id(message.chat.id)
                    .text("Por favor, forneça um nome após o comando /tickets.")
                    .build();
                if let Err(err) = bot.send_message(&send_message_params).await {
                    println!("Failed to send message: {:?}", err);
                }
                return;
            }
            let name = args[1];
            match read_tickets() {
                Ok(tickets_map) => {
                    if let Some(person_tickets) = tickets_map.get(name) {
                        let response = format!(
                            "💼 Your inventory:\n\n{}\n\n{}\n\n{}",
                            person_tickets.tickets,
                            person_tickets.vip_tickets,
                            person_tickets.selected_ticket
                        );
                        let send_message_params = SendMessageParams::builder()
                            .chat_id(message.chat.id)
                            .text(response)
                            .build();
                        if let Err(err) = bot.send_message(&send_message_params).await {
                            println!("Failed to send message: {:?}", err);
                        }
                    } else {
                        let response = format!("Nenhum ticket encontrado para {}.", name);
                        let send_message_params = SendMessageParams::builder()
                            .chat_id(message.chat.id)
                            .text(response)
                            .build();
                        if let Err(err) = bot.send_message(&send_message_params).await {
                            println!("Failed to send message: {:?}", err);
                        }
                    }
                }
                Err(err) => {
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(format!("Erro ao ler os tickets: {}", err))
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
            }
        } else if text.starts_with("/receitas") {
            let args: Vec<&str> = text.split_whitespace().collect();
            if args.len() < 2 {
                let send_message_params = SendMessageParams::builder()
                    .chat_id(message.chat.id)
                    .text("Por favor, forneça um nome após o comando /receitas.")
                    .build();
                if let Err(err) = bot.send_message(&send_message_params).await {
                    println!("Failed to send message: {:?}", err);
                }
                return;
            }
            let name = args[1];
            match read_receitas() {
                Ok(receitas_map) => {
                    if let Some(receita) = receitas_map.get(name) {
                        let response = format!("💼 Your inventory:\n\n{}", receita);
                        let send_message_params = SendMessageParams::builder()
                            .chat_id(message.chat.id)
                            .text(response)
                            .build();
                        if let Err(err) = bot.send_message(&send_message_params).await {
                            println!("Failed to send message: {:?}", err);
                        }
                    } else {
                        let response = format!("Nenhuma receita encontrada para {}.", name);
                        let send_message_params = SendMessageParams::builder()
                            .chat_id(message.chat.id)
                            .text(response)
                            .build();
                        if let Err(err) = bot.send_message(&send_message_params).await {
                            println!("Failed to send message: {:?}", err);
                        }
                    }
                }
                Err(err) => {
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(format!("Erro ao ler as receitas: {}", err))
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
            }
        } else if text.starts_with("/pecas") {
            let args: Vec<&str> = text.split_whitespace().collect();
            if args.len() < 2 {
                let send_message_params = SendMessageParams::builder()
                    .chat_id(message.chat.id)
                    .text("Por favor, forneça um nome após o comando /pecas.")
                    .build();
                if let Err(err) = bot.send_message(&send_message_params).await {
                    println!("Failed to send message: {:?}", err);
                }
                return;
            }
            let name = args[1];
            match read_pecas() {
                Ok(pecas_map) => {
                    if let Some(peca) = pecas_map.get(name) {
                        let response = format!("💼 Your inventory:\n\n{}", peca);
                        let send_message_params = SendMessageParams::builder()
                            .chat_id(message.chat.id)
                            .text(response)
                            .build();
                        if let Err(err) = bot.send_message(&send_message_params).await {
                            println!("Failed to send message: {:?}", err);
                        }
                    } else {
                        let response = format!("Nenhuma peça encontrada para {}.", name);
                        let send_message_params = SendMessageParams::builder()
                            .chat_id(message.chat.id)
                            .text(response)
                            .build();
                        if let Err(err) = bot.send_message(&send_message_params).await {
                            println!("Failed to send message: {:?}", err);
                        }
                    }
                }
                Err(err) => {
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(format!("Erro ao ler as peças: {}", err))
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
            }
        } else if text == "/claims" {
            match read_claims() {
                Ok(claims) => {
                    let mut response = "📜 Lista de Claims:\n\n\n".to_string();
                    if claims.is_empty() {
                        response.push_str("Nenhum papel reivindicado ainda.");
                    } else {
                        for (user, claim) in claims {
                            response.push_str(&format!("-- {} :\t{} {}\n\n", user, claim.role_name, claim.role_emoji));
                        }
                    }
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(response)
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
                Err(err) => {
                    println!("Failed to read claims: {}", err);
                }
            }
        } else if text.starts_with("/claim") {
            let nick = text.trim_start_matches("/claim").trim();
            if nick.is_empty() {
                let send_message_params = SendMessageParams::builder()
                    .chat_id(message.chat.id)
                    .text("Por favor, forneça um nick de papel após o comando /claim.")
                    .build();
                if let Err(err) = bot.send_message(&send_message_params).await {
                    println!("Failed to send message: {:?}", err);
                }
                return;
            }
            let user_name = message.from.as_ref().map_or("Unknown", |u| &u.first_name);

            match (read_papeis(), read_claims()) {
                (Ok(papeis), Ok(mut claims)) => {
                    if let Some(papel) = papeis.iter().find(|p| p.nicks.iter().any(|n| n.eq_ignore_ascii_case(nick))) {
                        let claim = Claim {
                            role_name: papel.name.clone(),
                            role_emoji: papel.emoji.clone(),
                        };
                        claims.insert(user_name.to_string(), claim);
                        if let Err(err) = write_claims(&claims) {
                            println!("Failed to write claims: {}", err);
                        } else {
                            let response = format!("{} reivindicou o papel: {} {}", user_name, papel.name, papel.emoji);
                            let send_message_params = SendMessageParams::builder()
                                .chat_id(message.chat.id)
                                .text(response)
                                .build();
                            if let Err(err) = bot.send_message(&send_message_params).await {
                                println!("Failed to send message: {:?}", err);
                            }
                        }
                    } else {
                        let response = format!("Papel com o nick '{}' não encontrado.", nick);
                        let send_message_params = SendMessageParams::builder()
                            .chat_id(message.chat.id)
                            .text(response)
                            .build();
                        if let Err(err) = bot.send_message(&send_message_params).await {
                            println!("Failed to send message: {:?}", err);
                        }
                    }
                }
                (Err(err), _) => println!("Failed to read papeis: {}", err),
                (_, Err(err)) => println!("Failed to read claims: {}", err),
            }
        } else if text == "/reset" {
            let claims: HashMap<String, Claim> = HashMap::new();
            if let Err(err) = write_claims(&claims) {
                println!("Failed to write claims: {}", err);
            } else {
                let send_message_params = SendMessageParams::builder()
                    .chat_id(message.chat.id)
                    .text("A lista de claims foi limpa.")
                    .build();
                if let Err(err) = bot.send_message(&send_message_params).await {
                    println!("Failed to send message: {:?}", err);
                }
            }
        } else if text == "/barbossa" || text == "/jack" || text == "/elizabeth" {
            let team_name = text.trim_start_matches('/');
            match read_team(team_name) {
                Ok(mut players) => {
                    players.sort_by(|a, b| b.points.cmp(&a.points));
                    let mut response = format!("🏆 Pontuação do Time {} 🏆\n\n", team_name.to_uppercase());
                    for (i, player) in players.iter().enumerate() {
                        response.push_str(&format!("{}. {} ({}): {} pontos\n", i + 1, player.name, player.user, player.points));
                    }
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(response)
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
                Err(err) => {
                    let send_message_params = SendMessageParams::builder()
                        .chat_id(message.chat.id)
                        .text(format!("Erro ao ler a pontuação do time: {}", err))
                        .build();
                    if let Err(err) = bot.send_message(&send_message_params).await {
                        println!("Failed to send message: {:?}", err);
                    }
                }
            }
        }
    }

    if let Some(new_chat_members) = message.new_chat_members {
        for user in new_chat_members {
            if let Ok(mut crew) = read_crew() {
                let new_member = CrewMember {
                    username: user.username.clone().unwrap_or_default(),
                    first_name: user.first_name.clone(),
                    is_crewmember: true,
                };
                crew.subs.push(new_member);
                if let Err(err) = write_crew(&crew) {
                    println!("Failed to write crew file: {}", err);
                }
            }

            let text = format!("Bem-vindo a bordo {}. O Holandês Voador agora é seu lar", user.first_name);
            let send_message_params = SendMessageParams::builder()
                .chat_id(message.chat.id)
                .text(text)
                .build();
            if let Err(err) = bot.send_message(&send_message_params).await {
                println!("Failed to send welcome message: {:?}", err);
            }
        }
    }
}

async fn ask_gemini(question: &str) -> Result<String, String> {
    let gemini_api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
    let gemini = Gemini::new(gemini_api_key);

    let calendar_context = fs::read_to_string("calendario.json").unwrap_or_default();
    let missions_context = fs::read_to_string("missoes.json").unwrap_or_default();
    let chat_log_context = fs::read_to_string("chat_log.txt").unwrap_or_default();
    let crew_context = fs::read_to_string("tripulantes.json").unwrap_or_default();

    let context = format!(
        "Contexto do Calendário:\n{}\n\nContexto das Missões:\n{}\n\nContexto do Chat:\n{}\n\nContexto da Tripulação:\n{}",
        calendar_context, missions_context, chat_log_context, crew_context
    );

    let response = gemini
        .generate_content()
        .with_user_message(format!(
            "Com base no seguinte contexto:\n\n{}\n\nVocê é Will Turner, Capitão do Holandês Voador, do filme Piratas do Caribe. Responda a seguinte pergunta como se você fosse Will Turner do filme Piratas do Caribe, em Português do Brasil. Seja criativo, e tente não narrar tanto: {}",
            context, question
        ))
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    if let Some(part) = response.candidates[0].content.parts.get(0) {
        if let Part::Text { text, .. } = part {
            return Ok(text.clone());
        }
    }

    Ok("".to_string())
}

use strum_macros::{Display, FromRepr, IntoStaticStr};

// see https://bnetdocs.org/packet/index?order=created-datetime-asc&pktapplayer%5B%5D=5
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, FromRepr, IntoStaticStr)]
pub enum W3gsPacket {
    #[strum(serialize = "WC3C_CLIENT_PING")]
    Wc3cClientPing = 0x00, // custom hello packet
    #[strum(serialize = "PING_FROM_HOST")]
    PingFromHost = 0x01,
    #[strum(serialize = "SLOTINFOJOIN")]
    SlotInfoJoin = 0x04,
    #[strum(serialize = "REJECTJOIN")]
    RejectJoin = 0x05,
    #[strum(serialize = "PLAYERINFO")]
    PlayerInfo = 0x06,
    #[strum(serialize = "PLAYERLEFT")]
    PlayerLeft = 0x07,
    #[strum(serialize = "PLAYERLOADED")]
    PlayerLoaded = 0x08,
    #[strum(serialize = "SLOTINFO")]
    SlotInfo = 0x09,
    #[strum(serialize = "COUNTDOWN_START")]
    CountdownStart = 0x0A,
    #[strum(serialize = "COUNTDOWN_END")]
    CountdownEnd = 0x0B,
    #[strum(serialize = "INCOMING_ACTION")]
    IncomingAction = 0x0C,
    #[strum(serialize = "CHAT_FROM_HOST")]
    ChatFromHost = 0x0F,
    #[strum(serialize = "START_LAG")]
    StartLag = 0x10,
    #[strum(serialize = "STOP_LAG")]
    StopLag = 0x11,
    #[strum(serialize = "LEAVERS")]
    Leavers = 0x1B,
    #[strum(serialize = "HOST_KICK_PLAYER")]
    HostKickPlayer = 0x1C,
    #[strum(serialize = "REQJOIN")]
    ReqJoin = 0x1E,
    #[strum(serialize = "LEAVEREQ")]
    LeaveReq = 0x21,
    #[strum(serialize = "GAMELOADED_SELF")]
    GameLoadedSelf = 0x23,
    #[strum(serialize = "OUTGOING_ACTION")]
    OutgoingAction = 0x26,
    #[strum(serialize = "OUTGOING_KEEPALIVE")]
    OutgoingKeepalive = 0x27,
    #[strum(serialize = "CHAT_TO_HOST")]
    ChatToHost = 0x28,
    #[strum(serialize = "DROPREQ")]
    DropReq = 0x29,
    #[strum(serialize = "SEARCHGAME")]
    SearchGame = 0x2F,
    #[strum(serialize = "GAMEINFO")]
    GameInfo = 0x30,
    #[strum(serialize = "CREATEGAME")]
    CreateGame = 0x31,
    #[strum(serialize = "REFRESHGAME")]
    RefreshGame = 0x32,
    #[strum(serialize = "DECREATEGAME")]
    DecreateGame = 0x33,
    #[strum(serialize = "PING_FROM_OTHERS")]
    PingFromOthers = 0x35,
    #[strum(serialize = "PONG_TO_OTHERS")]
    PongToOthers = 0x36,
    #[strum(serialize = "CLIENTINFO")]
    ClientInfo = 0x37,
    #[strum(serialize = "MAPCHECK")]
    MapCheck = 0x3D,
    #[strum(serialize = "STARTDOWNLOAD")]
    StartDownload = 0x3F,
    #[strum(serialize = "MAPSIZE")]
    MapSize = 0x42,
    #[strum(serialize = "MAPPART")]
    MapPart = 0x43,
    #[strum(serialize = "MAPPARTOK")]
    MapPartOk = 0x44,
    #[strum(serialize = "MAPPARTNOTOK")]
    MapPartNotOk = 0x45,
    #[strum(serialize = "PONG_TO_HOST")]
    PongToHost = 0x46,
    #[strum(serialize = "INCOMING_ACTION2")]
    IncomingAction2 = 0x48,
    #[strum(serialize = "UNKNOWN")]
    Unknown,
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum GameType {
    ReignOfChaos = u32::from_be_bytes(*b"WAR3"),
    FrozenThrone = u32::from_be_bytes(*b"W3XP"),
}

impl GameType {
    fn serialize(self) -> [u8; 4] {
        (self as u32).to_le_bytes()
    }

    pub fn label(self) -> &'static str {
        match self {
            GameType::ReignOfChaos => "Reign of Chaos",
            GameType::FrozenThrone => "Frozen Throne",
        }
    }
}

// 12..16 empty
// see https://bnetdocs.org/packet/434/w3gs-searchgame
pub fn searchgame_payload(game: GameType, version: u32) -> [u8; 16] {
    let mut p = [0u8; 16];
    p[0] = 0xf7;
    p[1] = 0x2f;
    p[2] = 0x10;
    p[3] = 0x00;
    p[4..8].copy_from_slice(&game.serialize());
    p[8..12].copy_from_slice(&version.to_le_bytes());
    p
}

pub fn is_w3gs(buf: &[u8]) -> bool {
    buf.len() > 1 && buf[0] == 0xf7
}

pub fn is_greeting(buf: &[u8]) -> bool {
    buf.len() == 4 && buf[0] == 0xf7 && buf[1] == 0x00
}

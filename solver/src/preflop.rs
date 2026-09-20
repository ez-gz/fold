//! Preflop ranges v1: 6-max, 100bb, 2.5bb opens (SB 3bb), low rake.
//! Hand-authored to follow the consensus shape of solver output (linear opens, polarised blind
//! 3-bets, mixed frequencies at the boundaries). NOT the output of our own preflop solve and not
//! copied from any published chart; treat boundary hands as +/- one notch. Later tokens override.
pub const VERSION: &str = "ranges-v1";

pub const UTG_OPEN: &str = "55+,A2s+,K8s+,Q9s+,J9s+,T9s,ATo+,KJo+,44:0.5,33:0.3,22:0.3,98s:0.5,87s:0.5,76s:0.5,65s:0.4,K7s:0.5,QJo:0.5,A9o:0.3";
pub const CO_OPEN: &str = "22+,A2s+,K5s+,Q8s+,J8s+,T8s+,97s+,86s+,76s,65s,54s,A9o+,KTo+,QTo+,JTo,K4s:0.5,Q7s:0.5,J7s:0.5,A8o:0.5,K9o:0.5";
pub const BTN_OPEN: &str = "22+,A2s+,K2s+,Q4s+,J6s+,T6s+,96s+,85s+,75s+,64s+,53s+,43s,A4o+,K8o+,Q9o+,J9o+,T8o+,98o,Q3s:0.5,Q2s:0.5,J5s:0.5,A3o:0.5,A2o:0.5,K7o:0.5,Q8o:0.5,87o:0.5";
pub const SB_OPEN: &str = "22+,A2s+,K4s+,Q6s+,J7s+,T7s+,96s+,86s+,75s+,65s,54s,A5o+,K9o+,Q9o+,J9o+,T9o,K3s:0.5,K2s:0.5,Q5s:0.5,A4o:0.5,A3o:0.5,K8o:0.5,98o:0.5";

pub const BB_3BET_VS_UTG: &str = "QQ+,AKs,AKo:0.6,JJ:0.4,AQs:0.4,A5s:0.6,A4s:0.4,KQs:0.2,87s:0.2,76s:0.2";
pub const BB_CALL_VS_UTG: &str = "JJ-22,AQs-A2s,K7s+,Q8s+,J8s+,T8s+,97s+,86s+,75s+,65s,54s,AQo,AJo,KQo,JJ:0.6,AQs:0.6,A5s:0.4,A4s:0.6,KQs:0.8,87s:0.8,76s:0.8,AKo:0.4,ATo:0.7,KJo:0.7,QJo:0.5,K6s-K2s,Q7s-Q5s,J7s,T7s,96s,85s,64s,KTo:0.4,A9o:0.3";
pub const BB_3BET_VS_CO: &str = "JJ+,AQs+,AKo,TT:0.5,AJs:0.5,A5s,A4s,KJs:0.4,KTs:0.4,QJs:0.3,T9s:0.3,98s:0.3,87s:0.3,76s:0.3,AQo:0.4,KQo:0.25";
pub const BB_CALL_VS_CO: &str = "TT-22,AJs-A2s,K2s+,Q5s+,J7s+,T7s+,96s+,85s+,75s+,64s+,54s,AQo-A8o,KTo+,QTo+,JTo,TT:0.5,AJs:0.5,A5s:0,A4s:0,KJs:0.6,KTs:0.6,QJs:0.7,T9s:0.7,98s:0.7,87s:0.7,76s:0.7,AQo:0.6,KQo:0.75,A7o:0.5,K9o,T9o:0.5,Q4s-Q2s,J6s-J4s:0.5,T6s,95s,A6o-A2o:0.5,Q9o:0.5,J9o:0.5,98o:0.5,K8o:0.4";
pub const BB_3BET_VS_BTN: &str = "TT+,AQs+,AKo,99:0.5,AJs:0.5,A5s,A4s,A3s:0.5,KTs:0.5,K9s:0.5,QTs:0.5,J9s:0.5,T8s:0.5,97s:0.5,86s:0.5,76s:0.5,65s:0.5,AQo:0.5,AJo:0.3,KQo:0.4,88:0.3,A2s:0.5,K8s:0.5,Q9s:0.5,J8s:0.5,T7s:0.3,ATo:0.3,KJo:0.3,A5o:0.3";
pub const BB_CALL_VS_BTN: &str = "99-22,AJs-A2s,K2s+,Q3s+,J5s+,T6s+,95s+,85s+,74s+,64s+,53s+,43s,AQo-A2o,K8o+,Q9o+,J9o+,T8o+,98o,87o,99:0.5,AJs:0.5,A5s:0,A4s:0,A3s:0.5,KTs:0.5,K9s:0.5,QTs:0.5,J9s:0.5,T8s:0.5,97s:0.5,86s:0.5,76s:0.5,65s:0.5,AQo:0.5,AJo:0.7,KQo:0.6,Q2s:0.5,K7o:0.5,76o:0.4";
pub const BB_CALL_VS_SB: &str = "TT-22,AJs-A2s,K2s+,Q2s+,J4s+,T6s+,95s+,85s+,74s+,64s+,53s+,43s,AJo-A2o,K5o+,Q8o+,J8o+,T8o+,97o+,87o,76o,TT:0.5,AJs:0.5,KQs:0.5,KJs:0.6,AJo:0.6,KQo:0.6,A5s:0.5,A4s:0.5,J3s:0.5,T5s:0.5";

pub const BTN_CALL_VS_CO: &str = "99-22,AJs-A8s,KTs+,QTs+,JTs,T9s,98s,87s,76s,TT:0.5,AQs:0.5,A5s:0.5,A4s:0.5,KQs:0.6,J9s:0.5,65s:0.5,AQo:0.5,AJo:0.5,KQo:0.6";
pub const BTN_CALL_VS_UTG: &str = "TT-22,AQs-ATs,KTs+,QTs+,JTs,T9s,98s,87s:0.6,76s:0.6,65s:0.4,JJ:0.5,A5s:0.5,AQo:0.5,KQo:0.4";

pub const BTN_CALL_3BET: &str = "JJ-44,AQs-A9s,A5s,A4s,K9s+,Q9s+,J9s+,T8s+,97s+,87s,76s,65s,AQo,KQo,QQ:0.3,33:0.5,22:0.5,AKs:0.2,A8s:0.5,54s:0.5,AJo:0.5,KJo:0.4,AKo:0.3";
pub const SB_3BET_VS_BTN: &str = "99+,ATs+,A5s,A4s,KTs+,QTs+,JTs,T9s,AJo+,KQo,88:0.5,77:0.5,A9s:0.5,A3s:0.5,K9s:0.5,J9s:0.5,98s:0.5,87s:0.5,76s:0.5,65s:0.5,ATo:0.4,KJo:0.5";
pub const BTN_3BET_VS_CO: &str = "JJ+,AQs+,AKo,A5s,A4s,TT:0.5,AJs:0.5,A3s:0.4,KQs:0.5,KJs:0.4,KTs:0.4,QJs:0.3,JTs:0.3,T9s:0.3,98s:0.3,87s:0.3,76s:0.4,65s:0.4,AQo:0.6,AJo:0.2,KQo:0.3";
pub const CO_CALL_3BET: &str = "JJ-55,AQs-ATs,KTs+,QTs+,JTs,T9s,98s,QQ:0.3,44:0.5,AKs:0.2,A5s:0.6,A4s:0.4,87s:0.6,76s:0.6,65s:0.4,AQo:0.7,KQo:0.5,AKo:0.3";

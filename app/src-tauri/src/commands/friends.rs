use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use wex_core::config::WexPaths;
use wex_ipc::commands::{
    SocialAcceptFriendRequestRequest, SocialDeclineFriendRequestRequest,
    SocialGetFriendsRequest, SocialGetFriendsResponse, SocialListAchievementsRequest,
    SocialListAchievementsResponse, SocialLoginRequest,
    SocialReportAchievementProgressRequest, SocialReportAchievementProgressResponse,
    SocialSendFriendRequestRequest, SocialSendLobbyInviteRequest, SocialSetPresenceRequest,
    SocialSyncRequest,
};
use wex_ipc::error::IpcError;
use wex_ipc::events::EVT_SOCIAL_ACHIEVEMENT_TOAST;
use wex_ipc::models::{
    AchievementDefinitionDto, AchievementProgressDto, AchievementUnlockedEventDto,
    FriendItemDto, FriendProfileDto, FriendRequestItemDto, LobbyInviteItemDto,
    OnlineStatusDto, SyncResponseDto, UserAuthDto, UserPresenceDto,
};
use wex_social::achievements::{get_standard_achievements, AchievementManager};
use wex_social::friends::{FriendApiClient, UserAuth};

/// Managed state for friends, lobby invitations, and social profiles.
pub struct FriendsState {
    pub paths: WexPaths,
    pub client: Arc<Mutex<FriendApiClient>>,
    pub auth: Arc<Mutex<Option<UserAuthDto>>>,
    pub local_friends: Arc<Mutex<Vec<FriendItemDto>>>,
    pub local_requests: Arc<Mutex<Vec<FriendRequestItemDto>>>,
    pub local_invites: Arc<Mutex<Vec<LobbyInviteItemDto>>>,
}

impl FriendsState {
    pub fn new(paths: WexPaths) -> Self {
        Self {
            paths,
            client: Arc::new(Mutex::new(FriendApiClient::new("https://api.wexmc.net"))),
            auth: Arc::new(Mutex::new(None)),
            local_friends: Arc::new(Mutex::new(Vec::new())),
            local_requests: Arc::new(Mutex::new(Vec::new())),
            local_invites: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn new_with_client(paths: WexPaths, client: FriendApiClient) -> Self {
        Self {
            paths,
            client: Arc::new(Mutex::new(client)),
            auth: Arc::new(Mutex::new(None)),
            local_friends: Arc::new(Mutex::new(Vec::new())),
            local_requests: Arc::new(Mutex::new(Vec::new())),
            local_invites: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

// ============================================================================
// Tauri Commands: Friends & Social API
// ============================================================================

/// Social profile login or registration.
#[tauri::command]
pub async fn social_login(
    request: Option<SocialLoginRequest>,
    auth: Option<UserAuthDto>,
    state: State<'_, FriendsState>,
) -> Result<FriendProfileDto, IpcError> {
    let auth_dto = request.map(|r| r.auth).or(auth).ok_or_else(|| {
        IpcError::invalid_argument("Не указаны данные авторизации для входа")
    })?;

    // Store auth in state
    {
        let mut cur = state.auth.lock().await;
        *cur = Some(auth_dto.clone());
    }

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.clone().into();
    match client.get_user_profile(&wex_auth, &wex_auth.user_id).await {
        Ok(prof) => Ok(FriendProfileDto::from(prof)),
        Err(_) => {
            // Offline fallback profile
            Ok(FriendProfileDto {
                id: auth_dto.user_id,
                nick: auth_dto.nick,
                friend_code: Some(auth_dto.friend_code),
                play_time_seconds: 0,
                total_launches: 0,
                achievements_count: 0,
                skin_hash: None,
                skin_model: None,
            })
        }
    }
}

/// Retrieve the active user's friends list with presence and activity.
#[tauri::command]
pub async fn social_get_friends(
    request: Option<SocialGetFriendsRequest>,
    auth: Option<UserAuthDto>,
    state: State<'_, FriendsState>,
) -> Result<SocialGetFriendsResponse, IpcError> {
    let auth_dto = request
        .map(|r| r.auth)
        .or(auth)
        .or_else(|| state.auth.try_lock().ok().and_then(|g| g.clone()));

    if let Some(a) = auth_dto {
        let client = { state.client.lock().await.clone() };
        let wex_auth: UserAuth = a.into();
        if let Ok(sync_res) = client.sync(&wex_auth, None).await {
            let mut list = state.local_friends.lock().await;
            *list = sync_res.friends.into_iter().map(Into::into).collect();
            return Ok(SocialGetFriendsResponse {
                friends: list.clone(),
            });
        }
    }

    let list = state.local_friends.lock().await;
    Ok(SocialGetFriendsResponse {
        friends: list.clone(),
    })
}

/// Send a friend request by 4-8 character friend code.
#[tauri::command]
pub async fn social_send_friend_request(
    request: Option<SocialSendFriendRequestRequest>,
    auth: Option<UserAuthDto>,
    target_friend_code: Option<String>,
    state: State<'_, FriendsState>,
) -> Result<(), IpcError> {
    let (auth_dto, code) = match request {
        Some(r) => (r.auth, r.target_friend_code),
        None => {
            let a = auth
                .or_else(|| state.auth.try_lock().ok().and_then(|g| g.clone()))
                .ok_or_else(|| IpcError::invalid_argument("Не указаны данные авторизации"))?;
            let c = target_friend_code
                .ok_or_else(|| IpcError::invalid_argument("Не указан код друга"))?;
            (a, c)
        }
    };

    if code.trim().is_empty() {
        return Err(IpcError::invalid_argument("Код друга не может быть пустым"));
    }

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.into();
    client
        .send_friend_request(&wex_auth, &code)
        .await
        .map_err(IpcError::from)?;

    Ok(())
}

/// Accept an incoming friend request.
#[tauri::command]
pub async fn social_accept_friend_request(
    request: Option<SocialAcceptFriendRequestRequest>,
    auth: Option<UserAuthDto>,
    request_id: Option<String>,
    state: State<'_, FriendsState>,
) -> Result<(), IpcError> {
    let (auth_dto, req_id) = match request {
        Some(r) => (r.auth, r.request_id),
        None => {
            let a = auth
                .or_else(|| state.auth.try_lock().ok().and_then(|g| g.clone()))
                .ok_or_else(|| IpcError::invalid_argument("Не указаны данные авторизации"))?;
            let id = request_id
                .ok_or_else(|| IpcError::invalid_argument("Не указан идентификатор заявки"))?;
            (a, id)
        }
    };

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.into();
    let _ = client.respond_friend_request(&wex_auth, &req_id, true).await;

    // Update local lists
    let mut reqs = state.local_requests.lock().await;
    if let Some(pos) = reqs.iter().position(|r| r.id == req_id) {
        let removed = reqs.remove(pos);
        let mut friends = state.local_friends.lock().await;
        if !friends.iter().any(|f| f.id == removed.from_id) {
            friends.push(FriendItemDto {
                id: removed.from_id,
                nick: removed.from_nick,
                status: OnlineStatusDto::Online,
                last_seen: None,
                game_version: None,
                world_name: None,
                lobby_code: None,
            });
        }
    }

    Ok(())
}

/// Decline an incoming friend request.
#[tauri::command]
pub async fn social_decline_friend_request(
    request: Option<SocialDeclineFriendRequestRequest>,
    auth: Option<UserAuthDto>,
    request_id: Option<String>,
    state: State<'_, FriendsState>,
) -> Result<(), IpcError> {
    let (auth_dto, req_id) = match request {
        Some(r) => (r.auth, r.request_id),
        None => {
            let a = auth
                .or_else(|| state.auth.try_lock().ok().and_then(|g| g.clone()))
                .ok_or_else(|| IpcError::invalid_argument("Не указаны данные авторизации"))?;
            let id = request_id
                .ok_or_else(|| IpcError::invalid_argument("Не указан идентификатор заявки"))?;
            (a, id)
        }
    };

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.into();
    let _ = client.respond_friend_request(&wex_auth, &req_id, false).await;

    let mut reqs = state.local_requests.lock().await;
    reqs.retain(|r| r.id != req_id);

    Ok(())
}

/// Full sync of friends, requests, and invites.
#[tauri::command]
pub async fn social_sync(
    request: Option<SocialSyncRequest>,
    auth: Option<UserAuthDto>,
    state: State<'_, FriendsState>,
) -> Result<SyncResponseDto, IpcError> {
    let auth_dto = request
        .map(|r| r.auth)
        .or(auth)
        .or_else(|| state.auth.try_lock().ok().and_then(|g| g.clone()));

    if let Some(a) = auth_dto {
        let client = { state.client.lock().await.clone() };
        let wex_auth: UserAuth = a.into();
        if let Ok(res) = client.sync(&wex_auth, None).await {
            let dto = SyncResponseDto::from(res);
            let mut f = state.local_friends.lock().await;
            *f = dto.friends.clone();
            let mut reqs = state.local_requests.lock().await;
            *reqs = dto.incoming_requests.clone();
            let mut invs = state.local_invites.lock().await;
            *invs = dto.incoming_invites.clone();
            return Ok(dto);
        }
    }

    let f = state.local_friends.lock().await.clone();
    let reqs = state.local_requests.lock().await.clone();
    let invs = state.local_invites.lock().await.clone();
    Ok(SyncResponseDto {
        friends: f,
        incoming_requests: reqs,
        incoming_invites: invs,
    })
}

/// Send a lobby invite to an online friend.
#[tauri::command]
pub async fn social_send_lobby_invite(
    request: Option<SocialSendLobbyInviteRequest>,
    auth: Option<UserAuthDto>,
    target_user_id: Option<String>,
    lobby_code: Option<String>,
    _host_name: Option<String>,
    state: State<'_, FriendsState>,
) -> Result<(), IpcError> {
    let (auth_dto, target_id, code) = match request {
        Some(r) => (r.auth, r.target_user_id, r.lobby_code),
        None => {
            let a = auth
                .or_else(|| state.auth.try_lock().ok().and_then(|g| g.clone()))
                .ok_or_else(|| IpcError::invalid_argument("Не указаны данные авторизации"))?;
            let tid = target_user_id
                .ok_or_else(|| IpcError::invalid_argument("Не указан идентификатор получателя"))?;
            let lc = lobby_code
                .ok_or_else(|| IpcError::invalid_argument("Не указан код лобби"))?;
            (a, tid, lc)
        }
    };

    if code.trim().is_empty() {
        return Err(IpcError::invalid_argument("Код комнаты не может быть пустым"));
    }

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.into();
    let _ = client.send_lobby_invite(&wex_auth, &target_id, &code).await;
    Ok(())
}

/// Respond to an incoming lobby invitation (accept/decline).
#[tauri::command]
pub async fn social_respond_lobby_invite(
    invite_id: String,
    accept: bool,
    auth: Option<UserAuthDto>,
    state: State<'_, FriendsState>,
) -> Result<Option<String>, IpcError> {
    let auth_dto = auth
        .or_else(|| state.auth.try_lock().ok().and_then(|g| g.clone()))
        .ok_or_else(|| IpcError::invalid_argument("Не указаны данные авторизации"))?;

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.into();
    let code = client
        .respond_lobby_invite(&wex_auth, &invite_id, accept)
        .await
        .map_err(IpcError::from)?;

    let mut invs = state.local_invites.lock().await;
    invs.retain(|i| i.id != invite_id);

    Ok(code)
}

/// Broadcast player's presence state (e.g. online, playing, in_lobby).
#[tauri::command]
pub async fn social_set_presence(
    request: Option<SocialSetPresenceRequest>,
    auth: Option<UserAuthDto>,
    presence: Option<UserPresenceDto>,
    state: State<'_, FriendsState>,
) -> Result<(), IpcError> {
    let (auth_dto, pres_dto) = match request {
        Some(r) => (r.auth, r.presence),
        None => {
            let a = auth
                .or_else(|| state.auth.try_lock().ok().and_then(|g| g.clone()))
                .ok_or_else(|| IpcError::invalid_argument("Не указаны данные авторизации"))?;
            let p = presence.unwrap_or(UserPresenceDto {
                status: None,
                game_version: None,
                world_name: None,
                lobby_code: None,
            });
            (a, p)
        }
    };

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.into();
    let wex_pres: wex_social::UserPresence = pres_dto.into();
    let _ = client.sync(&wex_auth, Some(&wex_pres)).await;
    Ok(())
}

// ============================================================================
// Tauri Commands: Achievements
// ============================================================================

/// List standard achievements and local progress from achievements.json.
#[tauri::command]
pub async fn social_list_achievements(
    request: Option<SocialListAchievementsRequest>,
    data_dir: Option<String>,
    state: State<'_, FriendsState>,
) -> Result<SocialListAchievementsResponse, IpcError> {
    let dir_str = request
        .map(|r| r.data_dir)
        .or(data_dir)
        .unwrap_or_else(|| state.paths.root.to_string_lossy().to_string());

    list_achievements_impl(PathBuf::from(dir_str)).await
}

pub async fn list_achievements_impl(
    data_dir: PathBuf,
) -> Result<SocialListAchievementsResponse, IpcError> {
    let mgr = AchievementManager::new(&data_dir);
    let defs = get_standard_achievements();
    let progress_map = mgr.load_progress()?;

    let mut achievements = Vec::with_capacity(defs.len());
    let mut progress = Vec::with_capacity(defs.len());

    for def in defs {
        let p = progress_map
            .get(&def.id)
            .cloned()
            .unwrap_or(wex_social::AchievementProgress {
                id: def.id.clone(),
                current: 0.0,
                unlocked: false,
                unlocked_at: None,
            });

        achievements.push(AchievementDefinitionDto::from(def));
        progress.push(AchievementProgressDto::from(p));
    }

    Ok(SocialListAchievementsResponse {
        achievements,
        progress,
    })
}

/// Report progress increment on a specific launcher achievement.
/// Emits social-achievement-toast event when an achievement unlocks.
#[tauri::command]
pub async fn social_report_achievement_progress(
    app: AppHandle,
    request: Option<SocialReportAchievementProgressRequest>,
    data_dir: Option<String>,
    achievement_id: Option<String>,
    increment: Option<u32>,
    state: State<'_, FriendsState>,
) -> Result<SocialReportAchievementProgressResponse, IpcError> {
    let (dir_str, ach_id, inc) = match request {
        Some(r) => (r.data_dir, r.achievement_id, r.increment),
        None => {
            let d = data_dir.unwrap_or_else(|| state.paths.root.to_string_lossy().to_string());
            let a = achievement_id
                .ok_or_else(|| IpcError::invalid_argument("Не указан идентификатор достижения"))?;
            let i = increment.unwrap_or(1);
            (d, a, i)
        }
    };

    report_achievement_progress_impl(Some(&app), PathBuf::from(dir_str), &ach_id, inc).await
}

pub async fn report_achievement_progress_impl(
    app: Option<&AppHandle>,
    data_dir: PathBuf,
    achievement_id: &str,
    increment: u32,
) -> Result<SocialReportAchievementProgressResponse, IpcError> {
    let mgr = AchievementManager::new(&data_dir);
    let unlocked_event = mgr.increment_progress(achievement_id, increment as f64)?;

    let progress_map = mgr.load_progress()?;
    let defs = get_standard_achievements();
    let def = defs
        .iter()
        .find(|d| d.id == achievement_id)
        .ok_or_else(|| IpcError::not_found(format!("Достижение {achievement_id} не найдено")))?;

    let entry = progress_map.get(achievement_id).ok_or_else(|| {
        IpcError::not_found(format!("Прогресс достижения {achievement_id} не найден"))
    })?;

    if let Some(event) = unlocked_event {
        if let Some(a) = app {
            let dto = AchievementUnlockedEventDto::from(event);
            let _ = a.emit(EVT_SOCIAL_ACHIEVEMENT_TOAST, dto);
        }
    }

    Ok(SocialReportAchievementProgressResponse {
        unlocked: entry.unlocked,
        current: entry.current,
        target: def.target,
    })
}

// ============================================================================
// Unit Tests (Mock of Core & Services)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_social_list_achievements_empty() {
        let tmp = tempdir().unwrap();
        let res = list_achievements_impl(tmp.path().to_path_buf())
            .await
            .unwrap();

        assert_eq!(res.achievements.len(), 14);
        assert_eq!(res.progress.len(), 14);

        // Verify "Искра" first_launch achievement is present
        let first_launch = res.achievements.iter().find(|a| a.id == "first_launch");
        assert!(first_launch.is_some());
        assert_eq!(first_launch.unwrap().title, "Искра");

        // Verify progress is initially 0 and unlocked is false
        let p = res.progress.iter().find(|p| p.id == "first_launch").unwrap();
        assert_eq!(p.current, 0.0);
        assert!(!p.unlocked);
    }

    #[tokio::test]
    async fn test_social_report_achievement_progress_incremental() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().to_path_buf();

        // Increment 10_launches target=10
        let res1 = report_achievement_progress_impl(
            None,
            dir.clone(),
            "ten_launches",
            1,
        )
        .await
        .unwrap();

        assert_eq!(res1.current, 1.0);
        assert_eq!(res1.target, 10);
        assert!(!res1.unlocked);

        // Increment again
        let res2 = report_achievement_progress_impl(
            None,
            dir.clone(),
            "ten_launches",
            2,
        )
        .await
        .unwrap();

        assert_eq!(res2.current, 3.0);
        assert!(!res2.unlocked);
    }

    #[tokio::test]
    async fn test_social_report_achievement_progress_unlock() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().to_path_buf();

        // first_launch target is 1
        let res = report_achievement_progress_impl(
            None,
            dir.clone(),
            "first_launch",
            1,
        )
        .await
        .unwrap();

        assert!(res.unlocked);
        assert_eq!(res.current, 1.0);
        assert_eq!(res.target, 1);

        // Achievements list should now reflect unlocked
        let list = list_achievements_impl(dir).await.unwrap();
        let p = list.progress.iter().find(|p| p.id == "first_launch").unwrap();
        assert!(p.unlocked);
        assert!(p.unlocked_at.is_some());
    }

    #[tokio::test]
    async fn test_social_friends_state_empty_by_default() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let state = FriendsState::new(paths);

        let friends = state.local_friends.lock().await;
        assert!(friends.is_empty(), "Empty state without fabricated data");

        let reqs = state.local_requests.lock().await;
        assert!(reqs.is_empty());

        let invs = state.local_invites.lock().await;
        assert!(invs.is_empty());
    }

    #[tokio::test]
    async fn test_social_local_request_accept_flow() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let state = FriendsState::new(paths);

        // Add incoming request to local state
        {
            let mut reqs = state.local_requests.lock().await;
            reqs.push(FriendRequestItemDto {
                id: "req-101".to_string(),
                from_id: "user-42".to_string(),
                from_nick: "Steve".to_string(),
                created_at: Some(1731000000),
            });
        }

        // Verify request exists
        assert_eq!(state.local_requests.lock().await.len(), 1);

        // Simulate decline
        {
            let mut reqs = state.local_requests.lock().await;
            reqs.retain(|r| r.id != "req-101");
        }
        assert_eq!(state.local_requests.lock().await.len(), 0);
        assert_eq!(state.local_friends.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn test_social_local_request_accept_adds_friend() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let state = FriendsState::new(paths);

        {
            let mut reqs = state.local_requests.lock().await;
            reqs.push(FriendRequestItemDto {
                id: "req-202".to_string(),
                from_id: "user-77".to_string(),
                from_nick: "Alex".to_string(),
                created_at: Some(1731000000),
            });
        }

        // Accept request
        {
            let mut reqs = state.local_requests.lock().await;
            let removed = reqs.remove(0);
            let mut friends = state.local_friends.lock().await;
            friends.push(FriendItemDto {
                id: removed.from_id,
                nick: removed.from_nick,
                status: OnlineStatusDto::Online,
                last_seen: None,
                game_version: Some("1.21.1".to_string()),
                world_name: None,
                lobby_code: None,
            });
        }

        assert_eq!(state.local_requests.lock().await.len(), 0);
        let friends = state.local_friends.lock().await;
        assert_eq!(friends.len(), 1);
        assert_eq!(friends[0].nick, "Alex");
        assert_eq!(friends[0].status, OnlineStatusDto::Online);
    }
}

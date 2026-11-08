<!--
  Apple HIG Profile Badge Component
  Specification: docs/SPEC.md 1.2
  Rules:
  - Avatar: player skin head when online, initial letter fallback when offline or error
  - Player nickname
  - Real network status indicator (online / offline)
  - Frameless window no-drag integration
  - Human Interface Guidelines: Toolbars & Windows
-->
<script lang="ts">
  interface Props {
    playerName?: string;
    isOnline?: boolean;
    skinHeadUrl?: string;
  }

  let {
    playerName = 'Player',
    isOnline = false,
    skinHeadUrl,
  }: Props = $props();

  let imageError = $state(false);

  let initial = $derived.by(() => {
    const trimmed = playerName.trim();
    return trimmed.length > 0 ? trimmed[0].toUpperCase() : '?';
  });

  let resolvedAvatarUrl = $derived.by(() => {
    if (!isOnline) return '';
    if (skinHeadUrl) return skinHeadUrl;
    const trimmed = playerName.trim();
    return trimmed ? `https://mc-heads.net/avatar/${encodeURIComponent(trimmed)}/32` : '';
  });

  // Reset image error state when player or online status changes
  $effect(() => {
    if (resolvedAvatarUrl) {
      imageError = false;
    }
  });

  function handleImageError() {
    imageError = true;
  }
</script>

<div
  class="apple-profile-badge"
  role="status"
  aria-label="Профиль игрока: {playerName}, {isOnline ? 'в сети' : 'не в сети'}"
  data-tauri-drag-region="false"
>
  <!-- Avatar: Head skin or initial letter -->
  <div class="avatar-capsule">
    {#if isOnline && resolvedAvatarUrl && !imageError}
      <img
        src={resolvedAvatarUrl}
        alt="Скин игрока {playerName}"
        class="avatar-head-image"
        onerror={handleImageError}
        loading="lazy"
      />
    {:else}
      <div class="avatar-letter-fallback" aria-hidden="true">
        {initial}
      </div>
    {/if}

    <!-- Network status dot badge -->
    <span
      class="network-status-dot"
      class:online={isOnline}
      class:offline={!isOnline}
      title={isOnline ? 'В сети' : 'Не в сети (офлайн)'}
      aria-label={isOnline ? 'В сети' : 'Офлайн'}
    ></span>
  </div>

  <!-- Player nickname and network label -->
  <div class="profile-info-text">
    <span class="profile-nickname">{playerName}</span>
    <span class="profile-status-caption" class:online={isOnline}>
      {isOnline ? 'В сети' : 'Офлайн'}
    </span>
  </div>
</div>

<style>
  .apple-profile-badge {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 3px 10px 3px 5px;
    border-radius: var(--radius-pill);
    background-color: var(--color-quaternary-system-fill);
    border: 1px solid var(--glass-border);
    user-select: none;
    -webkit-app-region: no-drag;
    cursor: default;
    transition: background-color var(--motion-duration-short) ease,
                border-color var(--motion-duration-short) ease;
  }

  .apple-profile-badge:hover {
    background-color: var(--color-tertiary-system-fill);
  }

  .avatar-capsule {
    position: relative;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .avatar-head-image {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    image-rendering: pixelated;
    object-fit: cover;
    background-color: var(--color-quaternary-system-fill);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12);
  }

  .avatar-letter-fallback {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background-color: var(--color-secondary-system-fill);
    color: var(--color-label);
    font-size: 13px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
    letter-spacing: -0.01em;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
  }

  .network-status-dot {
    position: absolute;
    bottom: -1px;
    right: -2px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: 1.5px solid var(--glass-regular-bg);
    box-sizing: border-box;
    transition: background-color var(--motion-duration-short) ease;
  }

  .network-status-dot.online {
    background-color: var(--color-system-green);
    box-shadow: 0 0 4px rgba(48, 209, 88, 0.4);
  }

  .network-status-dot.offline {
    background-color: var(--color-system-orange);
  }

  .profile-info-text {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    line-height: 1.15;
  }

  .profile-nickname {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-label);
    max-width: 120px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .profile-status-caption {
    font-size: 10px;
    font-weight: 500;
    color: var(--color-secondary-label);
  }

  .profile-status-caption.online {
    color: var(--color-system-green);
  }
</style>

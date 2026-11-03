
import "./App.css";
import { openUrl } from "@tauri-apps/plugin-opener";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  useCallback,
  useEffect,
  useState,
  type MouseEvent,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// ============================================================
// TYPES
// ============================================================

interface AccountInfo {
  username: string;
  uuid: string;
  skinUrl: string | null;
}

interface DeviceCodeInfo {
  user_code: string;
  verification_uri: string;
  message: string;
}

interface Profile {
  name: string;
  minecraftVersion: string;
  fabricVersion: string;
  modpackVersion: string;
  devMode: boolean;
  ramGb: number;
}

type LauncherStage =
  | "syncing"
  | "preparing"
  | "starting"
  | "running"
  | "stopped"
  | "error";

interface LauncherStatus {
  stage: LauncherStage;
  profileName: string;
  message: string;
}

const appWindow = getCurrentWindow();

// ============================================================
// HELPERS
// ============================================================

function getLauncherStageLabel(stage: LauncherStage): string {
  switch (stage) {
    case "syncing":
      return "Modpack-Synchronisierung";

    case "preparing":
      return "Installation wird vorbereitet";

    case "starting":
      return "Minecraft wird gestartet";

    case "running":
      return "Minecraft läuft";

    case "stopped":
      return "Minecraft beendet";

    case "error":
      return "Fehler";
  }
}

function getLauncherStageIcon(stage: LauncherStage): string {
  switch (stage) {
    case "syncing":
      return "↻";

    case "preparing":
      return "⚙";

    case "starting":
      return "▶";

    case "running":
      return "●";

    case "stopped":
      return "✓";

    case "error":
      return "!";
  }
}

// ============================================================
// MINECRAFT AVATAR
// ============================================================

function MinecraftAvatar({ account }: { account: AccountInfo }) {
  const [skinLoaded, setSkinLoaded] = useState(false);
  const [skinFailed, setSkinFailed] = useState(false);

  const showSkin = Boolean(account.skinUrl) && !skinFailed;

  return (
    <div className="account-avatar">
      {showSkin && (
        <div
          className="minecraft-avatar-skin"
          style={{
            backgroundImage: `url("${account.skinUrl}")`,
            visibility: skinLoaded ? "visible" : "hidden",
          }}
        >
          <img
            src={account.skinUrl!}
            alt=""
            onLoad={() => setSkinLoaded(true)}
            onError={() => setSkinFailed(true)}
          />
        </div>
      )}

      {(!showSkin || !skinLoaded) && (
        <span>{account.username.charAt(0).toUpperCase()}</span>
      )}
    </div>
  );
}

// ============================================================
// APP
// ============================================================

function App() {
  const [selectedProfile, setSelectedProfile] = useState("Vanilla");

  const [account, setAccount] = useState<AccountInfo | null>(null);
  const [deviceCode, setDeviceCode] = useState<DeviceCodeInfo | null>(null);

  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [isRestoringSession, setIsRestoringSession] = useState(true);
  const [isLaunching, setIsLaunching] = useState(false);

  const [statusMessage, setStatusMessage] = useState("");

  // ============================================================
  // LAUNCHER STATUS
  // ============================================================

  const [launcherStatus, setLauncherStatus] =
    useState<LauncherStatus | null>(null);

  const isGameRunning = launcherStatus?.stage === "running";

  // ============================================================
  // CUSTOM WINDOW TITLEBAR
  // ============================================================

  const [isMaximized, setIsMaximized] = useState(false);

  async function handleTitlebarMouseDown(
    event: MouseEvent<HTMLDivElement>
  ) {
    if (event.button !== 0) return;

    const target = event.target as HTMLElement;

    if (target.closest("button")) return;

    try {
      await appWindow.startDragging();
    } catch (error) {
      console.error("Fenster konnte nicht verschoben werden:", error);
    }
  }

  useEffect(() => {
    let isMounted = true;
    let unlisten: (() => void) | undefined;

    async function initializeWindowState() {
      try {
        const maximized = await appWindow.isMaximized();

        if (isMounted) {
          setIsMaximized(maximized);
        }

        const cleanup = await appWindow.onResized(async () => {
          const currentMaximized = await appWindow.isMaximized();

          if (isMounted) {
            setIsMaximized(currentMaximized);
          }
        });

        if (isMounted) {
          unlisten = cleanup;
        } else {
          cleanup();
        }
      } catch (error) {
        console.error(
          "Fensterstatus konnte nicht geladen werden:",
          error
        );
      }
    }

    void initializeWindowState();

    return () => {
      isMounted = false;
      unlisten?.();
    };
  }, []);

  async function toggleWindowMaximize() {
    try {
      await appWindow.toggleMaximize();
      setIsMaximized(await appWindow.isMaximized());
    } catch (error) {
      console.error(
        "Fensterstatus konnte nicht geändert werden:",
        error
      );
    }
  }

  // ============================================================
  // APP VERSION
  // ============================================================

  const [appVersion, setAppVersion] = useState("");

  useEffect(() => {
    getVersion()
      .then(setAppVersion)
      .catch((error) => {
        console.error(
          "Launcher-Version konnte nicht geladen werden:",
          error
        );
      });
  }, []);

  // ============================================================
  // UPDATES
  // ============================================================

  const [availableUpdate, setAvailableUpdate] = useState<Update | null>(null);
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [isInstallingUpdate, setIsInstallingUpdate] = useState(false);
  const [updateMessage, setUpdateMessage] = useState("");
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [isUpdatePanelDismissed, setIsUpdatePanelDismissed] = useState(false);

  const isUpdateError =
    updateMessage.toLowerCase().includes("fehlgeschlagen") ||
    updateMessage.toLowerCase().includes("konnte nicht");

  const updatePanelVariant = isInstallingUpdate
    ? "installing"
    : availableUpdate
      ? "available"
      : isUpdateError
        ? "error"
        : "current";

  const checkForUpdates = useCallback(async (manual = false) => {
    setIsCheckingUpdate(true);

    if (manual) {
      setIsUpdatePanelDismissed(false);
      setUpdateMessage("Suche nach Updates...");
    } else {
      setUpdateMessage("");
    }

    try {
      const update = await check();

      if (update) {
        setAvailableUpdate(update);
        setIsUpdatePanelDismissed(false);
        setUpdateMessage(`Version ${update.version} ist verfügbar.`);
      } else {
        setAvailableUpdate(null);

        setUpdateMessage(
          manual
            ? "Du verwendest bereits die aktuelle Version."
            : ""
        );
      }
    } catch (error) {
      console.error("Update-Prüfung fehlgeschlagen:", error);

      setAvailableUpdate(null);

      setUpdateMessage(
        manual
          ? "Update-Prüfung fehlgeschlagen: " + String(error)
          : ""
      );
    } finally {
      setIsCheckingUpdate(false);
    }
  }, []);

  useEffect(() => {
    void checkForUpdates();
  }, [checkForUpdates]);

  async function installUpdate() {
    if (!availableUpdate) return;

    setIsInstallingUpdate(true);
    setDownloadProgress(0);
    setUpdateMessage("Update wird heruntergeladen...");

    let downloadedBytes = 0;
    let totalBytes = 0;

    try {
      await availableUpdate.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            totalBytes = event.data.contentLength ?? 0;
            downloadedBytes = 0;
            break;

          case "Progress":
            downloadedBytes += event.data.chunkLength;

            if (totalBytes > 0) {
              setDownloadProgress(
                Math.min(
                  100,
                  Math.round((downloadedBytes / totalBytes) * 100)
                )
              );
            }
            break;

          case "Finished":
            setDownloadProgress(100);
            setUpdateMessage(
              "Update heruntergeladen. Installation startet..."
            );
            break;
        }
      });

      setUpdateMessage("Update wurde installiert.");
    } catch (error) {
      console.error("Update-Installation fehlgeschlagen:", error);

      setUpdateMessage(
        "Update konnte nicht installiert werden: " + String(error)
      );

      setIsInstallingUpdate(false);
    }
  }

  // ============================================================
  // PROFILES
  // ============================================================

  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [isSavingRam, setIsSavingRam] = useState(false);

  const selectedProfileData = profiles.find(
    (profile) => profile.name === selectedProfile
  );

  const usesDevAuth = selectedProfileData?.devMode ?? false;

  async function loadProfiles() {
    try {
      const result = await invoke<Profile[]>("get_profiles");

      setProfiles(result);

      setSelectedProfile((current) => {
        const currentExists = result.some(
          (profile) => profile.name === current
        );

        return currentExists ? current : result[0]?.name ?? "";
      });
    } catch (error) {
      console.error("Profile konnten nicht geladen werden:", error);
      setStatusMessage(String(error));
    }
  }

  function handleRamSliderChange(ramGb: number) {
    if (!selectedProfileData) return;

    setProfiles((currentProfiles) =>
      currentProfiles.map((profile) =>
        profile.name === selectedProfileData.name
          ? { ...profile, ramGb }
          : profile
      )
    );
  }

  async function saveProfileRam(profileName: string, ramGb: number) {
    setIsSavingRam(true);

    try {
      await invoke("update_profile_ram", {
        profileName,
        ramGb,
      });

      setStatusMessage(
        `RAM-Einstellung für ${profileName} gespeichert.`
      );
    } catch (error) {
      console.error(
        "RAM-Einstellung konnte nicht gespeichert werden:",
        error
      );

      setStatusMessage(
        `RAM-Einstellung konnte nicht gespeichert werden: ${String(error)}`
      );

      await loadProfiles();
    } finally {
      setIsSavingRam(false);
    }
  }

  // ============================================================
  // INITIAL DATA + PROFILE METADATA
  // ============================================================

  useEffect(() => {
    let isMounted = true;

    async function initializeApp() {
      await loadProfiles();

      if (!isMounted) return;

      try {
        await invoke("refresh_profile_metadata");

        if (!isMounted) return;

        await loadProfiles();
      } catch (error) {
        console.error(
          "Profil-Metadaten konnten nicht aktualisiert werden:",
          error
        );
      }
    }

    void initializeApp();

    return () => {
      isMounted = false;
    };
  }, []);

  // ============================================================
  // MICROSOFT SESSION RESTORATION
  // ============================================================

  useEffect(() => {
    let isMounted = true;

    async function restoreSession() {
      try {
        const restoredAccount = await invoke<AccountInfo | null>(
          "restore_microsoft_session"
        );

        if (!isMounted) return;

        setAccount(restoredAccount);

        if (restoredAccount) {
          console.log(
            "Microsoft-Sitzung wiederhergestellt:",
            restoredAccount.username
          );
        }
      } catch (error) {
        console.error(
          "Microsoft-Sitzung konnte nicht wiederhergestellt werden:",
          error
        );

        if (isMounted) {
          setAccount(null);

          setStatusMessage(
            "Gespeicherte Anmeldung konnte nicht wiederhergestellt werden. Bitte melde dich erneut an."
          );
        }
      } finally {
        if (isMounted) {
          setIsRestoringSession(false);
        }
      }
    }

    void restoreSession();

    return () => {
      isMounted = false;
    };
  }, []);

  // ============================================================
  // MICROSOFT DEVICE CODE EVENT
  // ============================================================

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let isMounted = true;

    async function registerListener() {
      const cleanup = await listen<DeviceCodeInfo>(
        "microsoft-device-code",
        (event) => {
          setDeviceCode(event.payload);
        }
      );

      if (isMounted) {
        unlisten = cleanup;
      } else {
        cleanup();
      }
    }

    void registerListener();

    return () => {
      isMounted = false;
      unlisten?.();
    };
  }, []);

  // ============================================================
  // LAUNCHER STATUS EVENT
  // ============================================================

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let isMounted = true;

    async function registerListener() {
      try {
        const cleanup = await listen<LauncherStatus>(
          "launcher-status",
          (event) => {
            const status = event.payload;

            setLauncherStatus(status);

            if (
              status.stage === "running" ||
              status.stage === "stopped" ||
              status.stage === "error"
            ) {
              setIsLaunching(false);
            }
          }
        );

        if (isMounted) {
          unlisten = cleanup;
        } else {
          cleanup();
        }
      } catch (error) {
        console.error(
          "Launcher-Status-Listener konnte nicht registriert werden:",
          error
        );
      }
    }

    void registerListener();

    return () => {
      isMounted = false;
      unlisten?.();
    };
  }, []);

  // ============================================================
  // MICROSOFT LOGIN
  // ============================================================

  async function loginMicrosoft() {
    setIsLoggingIn(true);
    setDeviceCode(null);
    setStatusMessage("");

    try {
      const result = await invoke<AccountInfo>("login_microsoft");

      setAccount(result);
      setDeviceCode(null);

      setStatusMessage("Microsoft-Anmeldung erfolgreich.");
    } catch (error) {
      console.error("Microsoft-Anmeldung fehlgeschlagen:", error);
      setStatusMessage(String(error));
      setDeviceCode(null);
    } finally {
      setIsLoggingIn(false);
    }
  }

  // ============================================================
  // LOGOUT
  // ============================================================

  async function logoutMicrosoft() {
    try {
      await invoke("logout_microsoft");

      setAccount(null);
      setDeviceCode(null);

      setStatusMessage("Du wurdest abgemeldet.");
    } catch (error) {
      console.error("Abmeldung fehlgeschlagen:", error);
      setStatusMessage(String(error));
    }
  }

  // ============================================================
  // MICROSOFT DEVICE CODE
  // ============================================================

  async function openMicrosoftPage() {
    if (!deviceCode) return;

    try {
      await openUrl(deviceCode.verification_uri);
    } catch (error) {
      console.error(
        "Microsoft-Seite konnte nicht geöffnet werden:",
        error
      );

      setStatusMessage(
        "Microsoft-Seite konnte nicht geöffnet werden: " + String(error)
      );
    }
  }

  async function copyDeviceCode() {
    if (!deviceCode) return;

    try {
      await navigator.clipboard.writeText(deviceCode.user_code);
      setStatusMessage("Anmeldecode wurde kopiert.");
    } catch (error) {
      console.error("Code konnte nicht kopiert werden:", error);
      setStatusMessage("Der Code konnte nicht kopiert werden.");
    }
  }

  // ============================================================
  // LAUNCH GAME
  // ============================================================

  async function launchGame() {
    if (isLaunching || isGameRunning) return;

    if (!selectedProfile) {
      setStatusMessage("Es wurde kein Spielprofil ausgewählt.");
      return;
    }

    if (!usesDevAuth && !account) {
      setStatusMessage("Bitte melde dich zuerst mit Microsoft an.");
      return;
    }

    setIsLaunching(true);
    setLauncherStatus(null);
    setStatusMessage("");

    try {
      await invoke<string>("launch_game", {
        profileName: selectedProfile,
      });

      await loadProfiles();
    } catch (error) {
      console.error("Minecraft konnte nicht gestartet werden:", error);
      setStatusMessage(String(error));
    } finally {
      setIsLaunching(false);
    }
  }

  // ============================================================
  // UI
  // ============================================================

  return (
    <div className="app-container">
      {/* CUSTOM TITLEBAR */}

      <div
        className="window-titlebar"
        onMouseDown={handleTitlebarMouseDown}
      >
        <div className="titlebar-brand">
          <img
            className="titlebar-logo"
            src="/icon.png"
            alt=""
            draggable={false}
          />

          <span className="titlebar-name">ForeverHub</span>
        </div>

        <div className="titlebar-controls">
          <button
            className="titlebar-button"
            onClick={() => void appWindow.minimize()}
            aria-label="Minimieren"
            title="Minimieren"
          >
            <span className="minimize-icon" />
          </button>

          <button
            className="titlebar-button"
            onClick={() => void toggleWindowMaximize()}
            aria-label={isMaximized ? "Wiederherstellen" : "Maximieren"}
            title={isMaximized ? "Wiederherstellen" : "Maximieren"}
          >
            <span className={isMaximized ? "restore-icon" : "maximize-icon"} />
          </button>

          <button
            className="titlebar-button close-button"
            onClick={() => void appWindow.close()}
            aria-label="Schließen"
            title="Schließen"
          >
            <span>×</span>
          </button>
        </div>
      </div>

      {/* HEADER */}

      <header className="app-header">
        <div className="header-info">
          <h1>ForeverHub</h1>

          <p className="app-subtitle">
            Der Minecraft-Launcher für die ForeverWorld
          </p>

          <p className="header-version">
            Version {appVersion || "Wird geladen..."}
          </p>
        </div>

        <div className="header-update">
          <button
            className="update-check-button"
            onClick={() => void checkForUpdates(true)}
            disabled={isCheckingUpdate || isInstallingUpdate}
          >
            <span
              className={`update-check-icon ${
                isCheckingUpdate ? "is-spinning" : ""
              }`}
              aria-hidden="true"
            >
              ↻
            </span>

            <span>
              {isCheckingUpdate ? "Suche läuft..." : "Nach Updates suchen"}
            </span>
          </button>
        </div>
      </header>

      {/* UPDATE PANEL */}

      {(availableUpdate || updateMessage) && !isUpdatePanelDismissed && (
        <section
          className={`update-panel update-panel-${updatePanelVariant}`}
          role="status"
          aria-live="polite"
        >
          <div className="update-status-icon" aria-hidden="true">
            {isUpdateError ? "!" : availableUpdate ? "↑" : "✓"}
          </div>

          <div className="update-info">
            <h3>
              {isInstallingUpdate
                ? "Update wird installiert"
                : isUpdateError
                  ? "Update fehlgeschlagen"
                  : availableUpdate
                    ? `Version ${availableUpdate.version} verfügbar`
                    : "Du bist auf dem neuesten Stand"}
            </h3>

            <p>{updateMessage}</p>

            {isInstallingUpdate && (
              <div className="update-progress">
                <div className="update-progress-track">
                  <div
                    className="update-progress-fill"
                    style={{ width: `${downloadProgress}%` }}
                  />
                </div>

                <span>{downloadProgress}%</span>
              </div>
            )}
          </div>

          <div className="update-actions">
            {availableUpdate && (
              <button
                className="update-install-button"
                onClick={installUpdate}
                disabled={isInstallingUpdate}
              >
                {isInstallingUpdate
                  ? "Wird installiert..."
                  : "Jetzt aktualisieren"}
              </button>
            )}

            <button
              className="update-dismiss-button"
              onClick={() => setIsUpdatePanelDismissed(true)}
              disabled={isInstallingUpdate}
              aria-label="Update-Meldung schließen"
              title="Meldung schließen"
            >
              ×
            </button>
          </div>
        </section>
      )}

      {/* TOP SECTION: ACCOUNT + LAUNCH */}

      <section className="top-section">
        {/* ACCOUNT */}

        <div className="account-section">
          {usesDevAuth ? (
            <div className="account-card">
              <div className="account-info">
                <div className="account-avatar">DEV</div>

                <div className="account-details">
                  <span className="account-status development">
                    Development-Modus aktiv
                  </span>

                  <h3>Offline-Testmodus</h3>

                  <p>
                    Dieses Profil verwendet automatisch eine lokale Testsession.
                    Eine Microsoft-Anmeldung ist nicht erforderlich.
                  </p>
                </div>
              </div>

              <span className="version-badge">
                DEVELOPMENT
              </span>
            </div>
          ) : isRestoringSession ? (
            <div className="account-card">
              <div className="account-info">
                <div className="account-avatar">...</div>

                <div className="account-details">
                  <span className="account-status">
                    Microsoft-Konto
                  </span>

                  <h3>Anmeldung wird wiederhergestellt...</h3>

                  <p>
                    Deine gespeicherte Sitzung wird im Hintergrund überprüft.
                    Du kannst ForeverHub währenddessen bereits verwenden.
                  </p>
                </div>
              </div>
            </div>
          ) : account ? (
            <div className="account-card">
              <div className="account-info">
                <MinecraftAvatar account={account} />

                <div className="account-details">
                  <span className="account-status">
                    Microsoft-Konto verbunden
                  </span>

                  <h3>{account.username}</h3>

                  <p>Erfolgreich mit Minecraft angemeldet.</p>
                </div>
              </div>

              <div className="account-actions">
                <button
                  className="secondary-button"
                  onClick={logoutMicrosoft}
                  disabled={isGameRunning}
                >
                  Abmelden
                </button>
              </div>
            </div>
          ) : (
            <div className="account-card">
              <div className="account-info">
                <div className="account-avatar">?</div>

                <div className="account-details">
                  <span className="account-status disconnected">
                    Nicht verbunden
                  </span>

                  <h3>Kein Konto angemeldet</h3>

                  <p>
                    Melde dich mit deinem Microsoft-Konto an,
                    um Minecraft zu spielen.
                  </p>
                </div>
              </div>

              <div className="account-actions">
                <button
                  onClick={loginMicrosoft}
                  disabled={isLoggingIn}
                >
                  {isLoggingIn
                    ? "Anmeldung läuft..."
                    : "Mit Microsoft anmelden"}
                </button>
              </div>
            </div>
          )}

          {/* DEVICE CODE */}

          {deviceCode && (
            <div className="device-code-card">
              <div className="device-code-header">
                <h3>Microsoft-Anmeldung</h3>

                <span className="account-status">
                  Warte auf Bestätigung
                </span>
              </div>

              <p>
                Öffne die Microsoft-Anmeldeseite und gib dort
                deinen persönlichen Gerätecode ein.
              </p>

              <div className="device-code">
                {deviceCode.user_code}
              </div>

              <div className="device-code-actions">
                <button onClick={copyDeviceCode}>
                  Code kopieren
                </button>

                <button
                  className="secondary-button"
                  onClick={openMicrosoftPage}
                >
                  Microsoft-Seite öffnen
                </button>
              </div>

              <p className="device-code-note">
                Nach der Bestätigung wirst du automatisch angemeldet.
              </p>
            </div>
          )}

          {/* GENERAL STATUS */}

          {statusMessage && (
            <div className="status-message">
              {statusMessage}
            </div>
          )}
        </div>

        {/* LAUNCH PANEL */}

        <section className="launch-panel">
          <div className="launch-info">
            <span className="launch-eyebrow">
              DEIN AKTIVES SPIELPROFIL
            </span>

            <h2>
              {selectedProfileData?.name ?? "Kein Profil ausgewählt"}
            </h2>

            {selectedProfileData && (
              <div className="launch-versions">
                <span>
                  Minecraft {selectedProfileData.minecraftVersion}
                </span>

                <span className="launch-separator">·</span>

                <span>
                  Fabric {selectedProfileData.fabricVersion}
                </span>

                <span className="launch-separator">·</span>

                <span>
                  Modpack {selectedProfileData.modpackVersion}
                </span>
              </div>
            )}

            <p>
              {isGameRunning
                ? `Minecraft läuft mit dem Profil ${launcherStatus.profileName}.`
                : usesDevAuth
                  ? "Development-Modus – lokale Testsession"
                  : isRestoringSession
                    ? "Microsoft-Sitzung wird im Hintergrund überprüft..."
                    : account
                      ? "Alles bereit. Du kannst Minecraft starten."
                      : "Melde dich mit Microsoft an, um Minecraft zu spielen."}
            </p>
          </div>

          <button
            className="launch-button"
            onClick={launchGame}
            disabled={
              (!usesDevAuth && isRestoringSession) ||
              isLaunching ||
              isGameRunning ||
              isSavingRam ||
              profiles.length === 0 ||
              (!usesDevAuth && !account)
            }
          >
            {isGameRunning
              ? "●  Minecraft läuft"
              : isLaunching
                ? "Minecraft wird gestartet..."
                : !usesDevAuth && isRestoringSession
                  ? "Anmeldung wird geprüft..."
                  : "▶  Minecraft starten"}
          </button>

          {/* LAUNCHER STATUS */}

          {launcherStatus && (
            <div
              className={`launcher-status launcher-status-${launcherStatus.stage}`}
              role="status"
              aria-live="polite"
            >
              <div className="launcher-status-icon" aria-hidden="true">
                {getLauncherStageIcon(launcherStatus.stage)}
              </div>

              <div className="launcher-status-content">
                <span className="launcher-status-label">
                  {getLauncherStageLabel(launcherStatus.stage)}
                </span>

                <p>{launcherStatus.message}</p>

                {launcherStatus.stage !== "stopped" &&
                  launcherStatus.stage !== "error" &&
                  launcherStatus.stage !== "running" && (
                    <div className="launcher-status-indicator">
                      <span />
                      <span />
                      <span />
                    </div>
                  )}
              </div>
            </div>
          )}
        </section>
      </section>

      {/* PROFILES */}

      <section className="profiles-section">
        <h2>Spielprofile</h2>

        <div className="profile-card-container">
          {profiles.length === 0 ? (
            <p>Spielprofile werden geladen...</p>
          ) : (
            profiles.map((profile) => (
              <div
                className={
                  selectedProfile === profile.name
                    ? "profile-card selected"
                    : "profile-card"
                }
                key={profile.name}
              >
                <div className="profile-card-heading">
                  <h2>{profile.name}</h2>

                  {profile.devMode && (
                    <span className="profile-dev-badge">
                      DEV
                    </span>
                  )}
                </div>

                <p>
                  Minecraft-Version:{" "}
                  <span className="version-badge">
                    {profile.minecraftVersion}
                  </span>
                </p>

                <p>
                  Fabric-Version:{" "}
                  <span className="version-badge">
                    {profile.fabricVersion}
                  </span>
                </p>

                <p>
                  Modpack-Version:{" "}
                  <span className="version-badge">
                    {profile.modpackVersion}
                  </span>
                </p>

                <button
                  className={
                    selectedProfile === profile.name
                      ? "selected-button"
                      : "profile-button"
                  }
                  onClick={() => {
                    setSelectedProfile(profile.name);
                    setStatusMessage("");
                  }}
                  disabled={isGameRunning || isLaunching}
                >
                  {selectedProfile === profile.name
                    ? "Ausgewählt"
                    : "Profil auswählen"}
                </button>
              </div>
            ))
          )}
        </div>

        {/* RAM SETTINGS FOR THE SELECTED PROFILE */}

        {selectedProfileData && (
          <div
            className="ram-control"
            style={{
              marginTop: "24px",
              width: "100%",
              maxWidth: "420px",
            }}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                gap: "12px",
                marginBottom: "8px",
              }}
            >
              <label
                htmlFor="profile-ram-slider"
                style={{ fontWeight: 600 }}
              >
                Zugewiesener Arbeitsspeicher
              </label>

              <strong aria-live="polite">
                {selectedProfileData.ramGb ?? 4} GB
              </strong>
            </div>

            <input
              id="profile-ram-slider"
              type="range"
              min={2}
              max={12}
              step={1}
              value={selectedProfileData.ramGb ?? 4}
              onChange={(event) =>
                handleRamSliderChange(Number(event.currentTarget.value))
              }
              onPointerUp={() => {
                void saveProfileRam(
                  selectedProfileData.name,
                  selectedProfileData.ramGb ?? 4
                );
              }}
              onKeyUp={() => {
                void saveProfileRam(
                  selectedProfileData.name,
                  selectedProfileData.ramGb ?? 4
                );
              }}
              disabled={isSavingRam || isLaunching || isGameRunning}
              aria-label="Minecraft-Arbeitsspeicher in Gigabyte"
              aria-valuetext={`${selectedProfileData.ramGb ?? 4} Gigabyte`}
              style={{ width: "100%", cursor: "pointer" }}
            />

            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                fontSize: "0.8rem",
                opacity: 0.7,
                marginTop: "4px",
              }}
            >
              <span>2 GB</span>
              <span>12 GB</span>
            </div>

            <p
              style={{
                marginTop: "8px",
                fontSize: "0.85rem",
                opacity: 0.75,
              }}
            >
              {isSavingRam
                ? "RAM-Einstellung wird gespeichert..."
                : "Die Einstellung gilt nur für dieses Profil."}
            </p>
          </div>
        )}
      </section>
    </div>
  );
}

export default App;


import "./App.css";
import { openUrl } from "@tauri-apps/plugin-opener";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
}

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

function App() {
  const [selectedProfile, setSelectedProfile] = useState("Vanilla");

  const [account, setAccount] = useState<AccountInfo | null>(null);
  const [deviceCode, setDeviceCode] = useState<DeviceCodeInfo | null>(null);

  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [isRestoringSession, setIsRestoringSession] = useState(true);
  const [isLaunching, setIsLaunching] = useState(false);

  const [statusMessage, setStatusMessage] = useState("");

  // ============================================================
  // APP VERSION
  // ============================================================

  const [appVersion, setAppVersion] = useState("");

  useEffect(() => {
    getVersion()
      .then(setAppVersion)
      .catch((error) => {
        console.error("Launcher-Version konnte nicht geladen werden:", error);
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

  async function checkForUpdates() {
    setIsCheckingUpdate(true);
    setUpdateMessage("");

    try {
      const update = await check();

      if (update) {
        setAvailableUpdate(update);
        setUpdateMessage(
          `Version ${update.version} ist verfügbar.`
        );
      } else {
        setAvailableUpdate(null);
        setUpdateMessage("Du verwendest bereits die aktuelle Version.");
      }
    } catch (error) {
      console.error("Update-Prüfung fehlgeschlagen:", error);
      setUpdateMessage(
        "Update-Prüfung fehlgeschlagen: " + String(error)
      );
    } finally {
      setIsCheckingUpdate(false);
    }
  }

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
            setUpdateMessage("Update heruntergeladen. Installation startet...");
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

    initializeApp();

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

    restoreSession();

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

    registerListener();

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
      console.error("Microsoft-Seite konnte nicht geöffnet werden:", error);
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
    if (!selectedProfile) {
      setStatusMessage("Es wurde kein Spielprofil ausgewählt.");
      return;
    }

    if (!usesDevAuth && !account) {
      setStatusMessage("Bitte melde dich zuerst mit Microsoft an.");
      return;
    }

    setIsLaunching(true);
    setStatusMessage("");

    try {
      const response = await invoke<string>("launch_game", {
        profileName: selectedProfile,
      });

      setStatusMessage(response);

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
            className="secondary-button"
            onClick={checkForUpdates}
            disabled={isCheckingUpdate || isInstallingUpdate}
          >
            {isCheckingUpdate
              ? "Suche nach Updates..."
              : "Nach Updates suchen"}
          </button>
        </div>
      </header>

      {/* UPDATE PANEL */}

      {(availableUpdate || updateMessage) && (
        <section className="update-panel">
          <div className="update-info">
            <h3>
              {availableUpdate
                ? `Update auf Version ${availableUpdate.version} verfügbar`
                : "Launcher-Updates"}
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

          {availableUpdate && (
            <button
              onClick={installUpdate}
              disabled={isInstallingUpdate}
            >
              {isInstallingUpdate
                ? "Update wird installiert..."
                : "Jetzt aktualisieren"}
            </button>
          )}
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

          {/* STATUS */}

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
              {usesDevAuth
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
              profiles.length === 0 ||
              (!usesDevAuth && !account)
            }
          >
            {isLaunching
              ? "Minecraft wird gestartet..."
              : !usesDevAuth && isRestoringSession
                ? "Anmeldung wird geprüft..."
                : "▶  Minecraft starten"}
          </button>
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
                >
                  {selectedProfile === profile.name
                    ? "Ausgewählt"
                    : "Profil auswählen"}
                </button>
              </div>
            ))
          )}
        </div>
      </section>
    </div>
  );
}

export default App;

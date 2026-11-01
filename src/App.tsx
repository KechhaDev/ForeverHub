
import "./App.css";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface AccountInfo {
  username: string;
  uuid: string;
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

function App() {
  const [selectedProfile, setSelectedProfile] = useState("Vanilla");

  const [account, setAccount] = useState<AccountInfo | null>(null);
  const [deviceCode, setDeviceCode] = useState<DeviceCodeInfo | null>(null);

  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [isLaunching, setIsLaunching] = useState(false);

  const [statusMessage, setStatusMessage] = useState("");

  // ============================================================
  // PROFILES
  // ============================================================

  const profiles: Profile[] = [
    {
      name: "Vanilla",
      minecraftVersion: "26.3",
      fabricVersion: "Nicht installiert",
      modpackVersion: "Vanilla",
      devMode: false
    },
    {
      name: "ForeverWorld",
      minecraftVersion: "26.3",
      fabricVersion: "0.19.5",
      modpackVersion: "1.0.0",
      devMode: false
    },
    {
      name: "ForeverWorld-DEVELOPMENT",
      minecraftVersion: "26.3",
      fabricVersion: "0.19.5",
      modpackVersion: "1.0.1-dev",
      devMode: true
    }
  ];

  const selectedProfileData = profiles.find(
    (profile) => profile.name === selectedProfile
  );

  const usesDevAuth = selectedProfileData?.devMode ?? false;

  // ============================================================
  // LOAD EXISTING ACCOUNT
  // ============================================================

  useEffect(() => {
    async function loadAccount() {
      try {
        const result = await invoke<AccountInfo | null>("get_account");
        setAccount(result);
      } catch (error) {
        console.error("Account konnte nicht geladen werden:", error);
      }
    }

    loadAccount();
  }, []);

  // ============================================================
  // MICROSOFT DEVICE CODE EVENT
  // ============================================================

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    async function registerListener() {
      unlisten = await listen<DeviceCodeInfo>(
        "microsoft-device-code",
        (event) => {
          setDeviceCode(event.payload);
        }
      );
    }

    registerListener();

    return () => {
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
  // COPY DEVICE CODE
  // ============================================================

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
    if (!usesDevAuth && !account) {
      setStatusMessage("Bitte melde dich zuerst mit Microsoft an.");
      return;
    }

    setIsLaunching(true);
    setStatusMessage("");

    try {
      const response = await invoke<string>("launch_game", {
        profileName: selectedProfile
      });

      setStatusMessage(response);
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

          <p className="header-status">
            Aktives Profil:{" "}
            <span className="version-badge">
              {selectedProfile}
            </span>
          </p>
        </div>

        <button
          className="launch-button"
          onClick={launchGame}
          disabled={isLaunching || (!usesDevAuth && !account)}
        >
          {isLaunching
            ? "Minecraft wird gestartet..."
            : "Minecraft starten"}
        </button>

      </header>

      {/* ACCOUNT */}

      <section className="account-section">

        <h2>
          {usesDevAuth ? "Development-Modus" : "Microsoft-Konto"}
        </h2>

        {usesDevAuth ? (
          <div className="account-card">

            <div className="account-info">
              <div className="account-avatar">
                DEV
              </div>

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
        ) : account ? (
          <div className="account-card">

            <div className="account-info">
              <div className="account-avatar">
                {account.username.charAt(0).toUpperCase()}
              </div>

              <div className="account-details">
                <span className="account-status">
                  Microsoft-Konto verbunden
                </span>

                <h3>{account.username}</h3>

                <p>
                  Erfolgreich mit Minecraft angemeldet.
                </p>
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
              <div className="account-avatar">
                ?
              </div>

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
                onClick={() => window.open(
                  deviceCode.verification_uri,
                  "_blank"
                )}
              >
                Microsoft-Seite öffnen
              </button>

            </div>

            <p className="device-code-note">
              {deviceCode.message}
            </p>

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

      </section>

      {/* PROFILES */}

      <section className="profiles-section">

        <h2>Spielprofile</h2>

        <div className="profile-card-container">

          {profiles.map((profile) => (
            <div
              className={
                selectedProfile === profile.name
                  ? "profile-card selected"
                  : "profile-card"
              }
              key={profile.name}
            >

              <h2>{profile.name}</h2>

              {profile.devMode && (
                <span className="version-badge">
                  DEVELOPMENT
                </span>
              )}

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
          ))}

        </div>

      </section>

    </div>
  );
}

export default App;

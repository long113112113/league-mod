import { getRouteApi } from "@tanstack/react-router";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { LuCircleAlert, LuCircleCheck, LuFolderOpen, LuInfo, LuLoader } from "react-icons/lu";

import { Button, IconButton } from "@/components/Button";
import { api, type Settings as SettingsType } from "@/lib/tauri";
import { useAppInfo, useSaveSettings, useSettings } from "@/modules/settings";
import { unwrapForQuery } from "@/utils/query";

const routeApi = getRouteApi("/settings");

export function Settings() {
  const { firstRun } = routeApi.useSearch();
  const { data: settings, isLoading } = useSettings();
  const { data: appInfo } = useAppInfo();
  const saveSettingsMutation = useSaveSettings();

  const [isDetecting, setIsDetecting] = useState(false);
  const [leaguePathValid, setLeaguePathValid] = useState<boolean | null>(null);

  useEffect(() => {
    if (settings?.leaguePath) {
      validatePath(settings.leaguePath);
    } else {
      setLeaguePathValid(null);
    }
  }, [settings?.leaguePath]);

  async function validatePath(path: string) {
    try {
      const result = await api.validateLeaguePath(path);
      setLeaguePathValid(unwrapForQuery(result));
    } catch {
      setLeaguePathValid(false);
    }
  }

  function saveSettings(newSettings: SettingsType) {
    saveSettingsMutation.mutate(newSettings);
  }

  async function handleAutoDetect() {
    if (!settings) return;

    setIsDetecting(true);
    try {
      const result = await api.autoDetectLeaguePath();
      const path = unwrapForQuery(result);
      if (path) {
        saveSettings({ ...settings, leaguePath: path, firstRunComplete: true });
      }
    } catch (error) {
      console.error("Failed to auto-detect:", error);
    } finally {
      setIsDetecting(false);
    }
  }

  async function handleBrowseLeaguePath() {
    if (!settings) return;

    try {
      const selected = await open({
        directory: true,
        title: "Select League of Legends Installation",
      });

      if (selected) {
        saveSettings({ ...settings, leaguePath: selected as string, firstRunComplete: true });
      }
    } catch (error) {
      console.error("Failed to browse:", error);
    }
  }



  if (isLoading || !settings) {
    return (
      <div className="flex h-full items-center justify-center">
        <LuLoader className="text-league-500 h-8 w-8 animate-spin" />
      </div>
    );
  }

  return (
    <div className="h-full overflow-auto">
      {/* Header */}
      <header className="flex h-16 items-center border-b border-surface-800 px-6">
        <h2 className="text-xl font-semibold text-surface-100">Settings</h2>
      </header>

      <div className="mx-auto max-w-2xl space-y-8 p-6">
        {/* First Run Banner */}
        {firstRun && !settings.leaguePath && (
          <div className="bg-league-500/10 border-league-500/30 flex items-start gap-3 rounded-lg border p-4">
            <LuInfo className="text-league-400 mt-0.5 h-5 w-5 shrink-0" />
            <div>
              <h3 className="text-league-300 font-medium">Welcome to LTK Manager!</h3>
              <p className="mt-1 text-sm text-surface-400">
                To get started, please configure your League of Legends installation path below. You
                can use auto-detection or browse to the folder manually.
              </p>
            </div>
          </div>
        )}

        {/* League Path */}
        <section>
          <h3 className="mb-4 text-lg font-medium text-surface-100">League of Legends</h3>
          <div className="space-y-3">
            <span className="block text-sm font-medium text-surface-400">Installation Path</span>
            <div className="flex gap-2">
              <div className="relative flex-1">
                <input
                  type="text"
                  value={settings.leaguePath || ""}
                  readOnly
                  placeholder="Not configured"
                  className="w-full rounded-lg border border-surface-700 bg-surface-800 px-4 py-2.5 text-surface-100 placeholder:text-surface-500"
                />
                {settings.leaguePath && (
                  <div className="absolute top-1/2 right-3 -translate-y-1/2">
                    {leaguePathValid === true && (
                      <LuCircleCheck className="h-5 w-5 text-green-500" />
                    )}
                    {leaguePathValid === false && (
                      <LuCircleAlert className="h-5 w-5 text-red-500" />
                    )}
                  </div>
                )}
              </div>
              <IconButton
                icon={<LuFolderOpen className="h-5 w-5" />}
                variant="outline"
                size="lg"
                onClick={handleBrowseLeaguePath}
              />
            </div>
            <Button
              variant="transparent"
              size="sm"
              onClick={handleAutoDetect}
              loading={isDetecting}
              left={isDetecting ? undefined : <LuLoader className="h-4 w-4" />}
              className="text-brand-400 hover:text-brand-300"
            >
              Auto-detect installation
            </Button>
            {leaguePathValid === false && settings.leaguePath && (
              <p className="text-sm text-red-400">
                Could not find League of Legends at this path. Make sure it points to the folder
                containing the <code className="rounded bg-surface-700 px-1">Game</code> directory.
              </p>
            )}
          </div>
        </section>



        {/* Workspace Path */}
        <section>
          <h3 className="mb-4 text-lg font-medium text-surface-100">Workspace</h3>
          <div className="space-y-3">
            <span className="block text-sm font-medium text-surface-400">Workspace Path</span>
            <div className="flex gap-2">
              <input
                type="text"
                value={settings.workspacePath || ""}
                readOnly
                placeholder="Not configured"
                className="flex-1 rounded-lg border border-surface-700 bg-surface-800 px-4 py-2.5 text-surface-100 placeholder:text-surface-500"
              />
              <IconButton
                icon={<LuFolderOpen className="h-5 w-5" />}
                variant="outline"
                size="lg"
                onClick={async () => {
                  if (!settings) return;
                  try {
                    const selected = await open({
                      directory: true,
                      title: "Select Workspace Directory",
                    });
                    if (selected) {
                      saveSettings({ ...settings, workspacePath: selected as string });
                    }
                  } catch (error) {
                    console.error("Failed to browse:", error);
                  }
                }}
              />
            </div>
            <p className="text-sm text-surface-500">
              Choose a directory for storing skin IDs, cache files, and other working data. This is
              required for the skin database feature.
            </p>
          </div>
        </section>



        {/* Theme */}
        <section>
          <h3 className="mb-4 text-lg font-medium text-surface-100">Appearance</h3>
          <div className="space-y-3">
            <span className="block text-sm font-medium text-surface-400">Theme</span>
            <div className="flex gap-2">
              {(["system", "dark", "light"] as const).map((theme) => (
                <Button
                  key={theme}
                  variant={settings.theme === theme ? "filled" : "default"}
                  size="sm"
                  onClick={() => saveSettings({ ...settings, theme })}
                  className="capitalize"
                >
                  {theme}
                </Button>
              ))}
            </div>
          </div>
        </section>

        {/* Data Management */}
        <section>
          <h3 className="mb-4 text-lg font-medium text-surface-100">Data Management</h3>
          <div className="space-y-3">
            <span className="block text-sm font-medium text-surface-400">Skin Database</span>
            <div className="rounded-lg border border-surface-800 bg-surface-900 p-4">
              <div className="flex items-center justify-between">
                <div>
                  <h4 className="font-medium text-surface-100">Champion & Skin Data</h4>
                  <p className="text-sm text-surface-500">
                    Update the local database of champions and skins from the remote repository.
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={async () => {
                    try {
                      const result = await api.refreshSkinDatabase();
                      const update = unwrapForQuery(result);
                      if (update.success) {
                        // Fetch champion data to verify
                        const championsResult = await api.getChampionsWithSkins();
                        const champions = unwrapForQuery(championsResult);
                        console.log(`Loaded ${champions.length} champions with skins`);
                        alert(`${update.message}\nLoaded ${champions.length} champions!`);
                      }
                    } catch (error) {
                      console.error("Failed to update skin database:", error);
                      alert("Failed to update skin database. Check console for details.");
                    }
                  }}
                >
                  Update Database
                </Button>
              </div>
            </div>
          </div>
        </section>

        {/* About */}
        <section>
          <h3 className="mb-4 text-lg font-medium text-surface-100">About</h3>
          <div className="rounded-lg border border-surface-800 bg-surface-900 p-4">
            <div className="flex items-center justify-between">
              <div>
                <h4 className="font-medium text-surface-100">LTK Manager</h4>
                {appInfo && <p className="text-sm text-surface-500">Version {appInfo.version}</p>}
              </div>
            </div>
            <p className="mt-3 text-sm text-surface-400">
              LTK Manager is part of the LeagueToolkit project. It provides a graphical interface
              for managing League of Legends mods using the modpkg format.
            </p>
            <div className="mt-4 flex gap-4 border-t border-surface-800 pt-4">
              <a
                href="https://github.com/LeagueToolkit/league-mod"
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-brand-400 transition-colors hover:text-brand-300"
              >
                View on GitHub →
              </a>
              <a
                href="https://github.com/LeagueToolkit/league-mod/wiki"
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-brand-400 transition-colors hover:text-brand-300"
              >
                Documentation →
              </a>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}

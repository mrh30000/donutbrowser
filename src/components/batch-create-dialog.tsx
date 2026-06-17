"use client";

import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { LuCheck, LuChevronsUpDown } from "react-icons/lu";
import { FingerprintConfigForm } from "@/components/fingerprint-config-form";
import { LoadingButton } from "@/components/loading-button";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { useBrowserDownload } from "@/hooks/use-browser-download";
import { useProxyEvents } from "@/hooks/use-proxy-events";
import { useVpnEvents } from "@/hooks/use-vpn-events";
import { cn } from "@/lib/utils";
import type {
  BrowserReleaseTypes,
  CamoufoxOS,
  FingerprintProfile,
} from "@/types";

const getCurrentOS = (): CamoufoxOS => {
  if (typeof navigator === "undefined") return "linux";
  const platform = navigator.platform.toLowerCase();
  if (platform.includes("win")) return "windows";
  if (platform.includes("mac")) return "macos";
  return "linux";
};

interface BatchCreateDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onCreated: () => void;
  selectedGroupId?: string;
}

const MAX_BATCH_COUNT = 50;

export function BatchCreateDialog({
  isOpen,
  onClose,
  onCreated,
  selectedGroupId,
}: BatchCreateDialogProps) {
  const { t } = useTranslation();

  const [count, setCount] = useState(5);
  const [namePrefix, setNamePrefix] = useState("");
  const [selectedProxyId, setSelectedProxyId] = useState<string>();
  const [proxyPopoverOpen, setProxyPopoverOpen] = useState(false);
  const [enableFingerprint, setEnableFingerprint] = useState(false);
  const [fingerprintProfile, setFingerprintProfile] =
    useState<FingerprintProfile>({});
  const [isCreating, setIsCreating] = useState(false);
  const [progress, setProgress] = useState({ current: 0, total: 0 });

  const [releaseTypes, setReleaseTypes] = useState<BrowserReleaseTypes>();
  const { storedProxies } = useProxyEvents();
  const { vpnConfigs } = useVpnEvents();
  const {
    isBrowserDownloading,
    loadDownloadedVersions,
    isVersionDownloaded,
    downloadedVersionsMap,
  } = useBrowserDownload();

  const loadReleaseTypes = useCallback(async () => {
    try {
      const raw = await invoke<BrowserReleaseTypes>(
        "get_browser_release_types",
        {
          browserStr: "camoufox",
        },
      );
      await loadDownloadedVersions("camoufox");
      const filtered: BrowserReleaseTypes = {};
      if (raw.stable) filtered.stable = raw.stable;
      setReleaseTypes(filtered);
    } catch {
      try {
        const downloaded = await loadDownloadedVersions("camoufox");
        if (downloaded.length > 0) {
          setReleaseTypes({ stable: downloaded[0] });
        }
      } catch {
        // ignore
      }
    }
  }, [loadDownloadedVersions]);

  useEffect(() => {
    if (isOpen) {
      void loadReleaseTypes();
      void loadDownloadedVersions("camoufox");
    }
  }, [isOpen, loadReleaseTypes, loadDownloadedVersions]);

  const getCreatableVersion = useCallback(() => {
    if (releaseTypes?.stable) {
      const v = releaseTypes.stable;
      if (isVersionDownloaded(v))
        return { version: v, releaseType: "stable" as const };
    }
    const downloaded = downloadedVersionsMap.camoufox ?? [];
    if (downloaded.length > 0) {
      return { version: downloaded[0], releaseType: "stable" as const };
    }
    return null;
  }, [releaseTypes, isVersionDownloaded, downloadedVersionsMap]);

  const handleCreate = async () => {
    if (!namePrefix.trim() || count < 1) return;
    const version = getCreatableVersion();
    if (!version) return;

    const isVpn = selectedProxyId?.startsWith("vpn-") ?? false;
    const proxyId = isVpn ? undefined : selectedProxyId;
    const vpnId =
      isVpn && selectedProxyId ? selectedProxyId.slice(4) : undefined;
    const fpProfile = enableFingerprint ? fingerprintProfile : undefined;

    setIsCreating(true);
    setProgress({ current: 0, total: count });

    const groupId =
      selectedGroupId && selectedGroupId !== "__all__"
        ? selectedGroupId
        : undefined;

    let successCount = 0;
    for (let i = 0; i < count; i++) {
      const name = `${namePrefix.trim()}-${String(i + 1).padStart(String(count).length, "0")}`;
      try {
        await invoke("create_browser_profile_new", {
          name,
          browserStr: "camoufox",
          version: version.version,
          releaseType: version.releaseType,
          proxyId,
          vpnId,
          camoufoxConfig: { os: getCurrentOS() },
          fingerprintProfile: fpProfile,
          groupId,
          ephemeral: false,
          dnsBlocklist: undefined,
          launchHook: undefined,
        });
        successCount++;
      } catch (err) {
        console.error(`Failed to create profile ${name}:`, err);
      }
      setProgress({ current: i + 1, total: count });
    }

    setIsCreating(false);
    if (successCount > 0) {
      onCreated();
    }
    handleClose();
  };

  const handleClose = () => {
    setNamePrefix("");
    setCount(5);
    setSelectedProxyId(undefined);
    setEnableFingerprint(false);
    setFingerprintProfile({});
    setProgress({ current: 0, total: 0 });
    onClose();
  };

  const canCreate =
    namePrefix.trim().length > 0 &&
    count >= 1 &&
    count <= MAX_BATCH_COUNT &&
    !isCreating &&
    !!getCreatableVersion() &&
    !isBrowserDownloading("camoufox");

  return (
    <Dialog open={isOpen} onOpenChange={handleClose}>
      <DialogContent className="w-[480px] max-w-[480px] max-h-[90vh] flex flex-col">
        <DialogHeader className="shrink-0">
          <DialogTitle>{t("batchCreate.title")}</DialogTitle>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto space-y-6 py-4">
          {/* Count */}
          <div className="space-y-2">
            <Label htmlFor="batch-count">{t("batchCreate.count")}</Label>
            <Input
              id="batch-count"
              type="number"
              min={1}
              max={MAX_BATCH_COUNT}
              value={count}
              onChange={(e) => {
                const v = parseInt(e.target.value, 10);
                if (!isNaN(v))
                  setCount(Math.min(Math.max(v, 1), MAX_BATCH_COUNT));
              }}
              disabled={isCreating}
            />
            <p className="text-xs text-muted-foreground">
              {t("batchCreate.countDescription", { max: MAX_BATCH_COUNT })}
            </p>
          </div>

          {/* Name prefix */}
          <div className="space-y-2">
            <Label htmlFor="batch-name-prefix">
              {t("batchCreate.namePrefix")}
            </Label>
            <Input
              id="batch-name-prefix"
              value={namePrefix}
              onChange={(e) => setNamePrefix(e.target.value)}
              placeholder={t("batchCreate.namePrefixPlaceholder")}
              disabled={isCreating}
            />
            <p className="text-xs text-muted-foreground">
              {t("batchCreate.namePrefixDescription")}
            </p>
          </div>

          {/* Proxy / VPN */}
          <div className="space-y-2">
            <Label>{t("createProfile.proxy.title")}</Label>
            {storedProxies.length > 0 || vpnConfigs.length > 0 ? (
              <Popover
                open={proxyPopoverOpen}
                onOpenChange={setProxyPopoverOpen}
              >
                <PopoverTrigger asChild>
                  <Button
                    variant="outline"
                    role="combobox"
                    aria-expanded={proxyPopoverOpen}
                    className="w-full justify-between font-normal"
                  >
                    {(() => {
                      if (!selectedProxyId)
                        return t("createProfile.proxy.noProxy");
                      if (selectedProxyId.startsWith("vpn-")) {
                        const vpn = vpnConfigs.find(
                          (v) => v.id === selectedProxyId.slice(4),
                        );
                        return vpn
                          ? `WG — ${vpn.name}`
                          : t("createProfile.proxy.noProxy");
                      }
                      const proxy = storedProxies.find(
                        (p) => p.id === selectedProxyId,
                      );
                      return proxy?.name ?? t("createProfile.proxy.noProxy");
                    })()}
                    <LuChevronsUpDown className="ml-2 size-4 shrink-0 opacity-50" />
                  </Button>
                </PopoverTrigger>
                <PopoverContent className="w-[340px] p-0" sideOffset={8}>
                  <Command>
                    <CommandInput
                      placeholder={t("createProfile.proxy.search")}
                    />
                    <CommandList>
                      <CommandEmpty>
                        {t("createProfile.proxy.notFound")}
                      </CommandEmpty>
                      <CommandGroup>
                        <CommandItem
                          value="__none__"
                          onSelect={() => {
                            setSelectedProxyId(undefined);
                            setProxyPopoverOpen(false);
                          }}
                        >
                          <LuCheck
                            className={cn(
                              "mr-2 size-4",
                              !selectedProxyId ? "opacity-100" : "opacity-0",
                            )}
                          />
                          {t("common.labels.none")}
                        </CommandItem>
                        {storedProxies.map((proxy) => (
                          <CommandItem
                            key={proxy.id}
                            value={proxy.name}
                            onSelect={() => {
                              setSelectedProxyId(proxy.id);
                              setProxyPopoverOpen(false);
                            }}
                          >
                            <LuCheck
                              className={cn(
                                "mr-2 size-4",
                                selectedProxyId === proxy.id
                                  ? "opacity-100"
                                  : "opacity-0",
                              )}
                            />
                            {proxy.name}
                          </CommandItem>
                        ))}
                      </CommandGroup>
                      {vpnConfigs.length > 0 && (
                        <CommandGroup heading="VPNs">
                          {vpnConfigs.map((vpn) => (
                            <CommandItem
                              key={vpn.id}
                              value={`vpn-${vpn.name}`}
                              onSelect={() => {
                                setSelectedProxyId(`vpn-${vpn.id}`);
                                setProxyPopoverOpen(false);
                              }}
                            >
                              <LuCheck
                                className={cn(
                                  "mr-2 size-4",
                                  selectedProxyId === `vpn-${vpn.id}`
                                    ? "opacity-100"
                                    : "opacity-0",
                                )}
                              />
                              <Badge
                                variant="outline"
                                className="text-[10px] px-1 py-0 leading-tight mr-1"
                              >
                                WG
                              </Badge>
                              {vpn.name}
                            </CommandItem>
                          ))}
                        </CommandGroup>
                      )}
                    </CommandList>
                  </Command>
                </PopoverContent>
              </Popover>
            ) : (
              <div className="flex gap-3 items-center p-3 text-sm rounded-md border text-muted-foreground">
                {t("createProfile.proxy.noProxiesAvailable")}
              </div>
            )}
          </div>

          {/* Fingerprint profile */}
          <div className="space-y-3 p-4 border rounded-lg bg-muted/30">
            <div className="flex items-center gap-x-2">
              <Checkbox
                id="batch-enable-fingerprint"
                checked={enableFingerprint}
                onCheckedChange={(checked) => {
                  setEnableFingerprint(checked === true);
                  if (checked !== true) {
                    setFingerprintProfile({});
                  }
                }}
                disabled={isCreating}
              />
              <Label htmlFor="batch-enable-fingerprint" className="font-medium">
                {t("fingerprintProfile.advancedConfig")}
              </Label>
            </div>
            <p className="text-sm text-muted-foreground ml-6">
              {t("batchCreate.sharedFingerprintDescription")}
            </p>
          </div>
          {enableFingerprint && (
            <FingerprintConfigForm
              profile={fingerprintProfile}
              onChange={(updates) => {
                setFingerprintProfile((prev) => ({ ...prev, ...updates }));
              }}
              readOnly={isCreating}
            />
          )}

          {/* Progress */}
          {isCreating && (
            <div className="space-y-2">
              <div className="flex justify-between text-sm text-muted-foreground">
                <span>{t("batchCreate.creating")}</span>
                <span>
                  {progress.current} / {progress.total}
                </span>
              </div>
              <div className="w-full h-2 bg-muted rounded-full overflow-hidden">
                <div
                  className="h-full bg-primary transition-all duration-200"
                  style={{
                    width: `${(progress.current / progress.total) * 100}%`,
                  }}
                />
              </div>
            </div>
          )}
        </div>

        <DialogFooter className="shrink-0 pt-4 border-t">
          <Button variant="outline" onClick={handleClose} disabled={isCreating}>
            {t("common.buttons.cancel")}
          </Button>
          <LoadingButton
            onClick={handleCreate}
            isLoading={isCreating}
            disabled={!canCreate}
          >
            {isCreating
              ? t("batchCreate.creatingButton", {
                  current: progress.current,
                  total: progress.total,
                })
              : t("batchCreate.createButton", { count })}
          </LoadingButton>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

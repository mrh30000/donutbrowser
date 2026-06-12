"use client";

import { useTranslation } from "react-i18next";
import { AnimatedSwitch as Switch } from "@/components/ui/animated-switch";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type {
  AutoOrManual,
  FingerprintProfile,
  GeoPermission,
  NoiseMode,
  WebRtcMode,
} from "@/types";

interface FingerprintConfigFormProps {
  profile: FingerprintProfile;
  onChange: (updates: Partial<FingerprintProfile>) => void;
  className?: string;
  readOnly?: boolean;
}

export function FingerprintConfigForm({
  profile,
  onChange,
  className = "",
  readOnly = false,
}: FingerprintConfigFormProps) {
  const { t } = useTranslation();

  const updateField = <K extends keyof FingerprintProfile>(
    key: K,
    value: FingerprintProfile[K],
  ) => {
    onChange({ [key]: value });
  };

  const uaMode = profile.user_agent?.mode ?? "Manual";
  const uaValue = profile.user_agent?.value ?? "";
  const langMode = profile.language?.mode ?? "Manual";
  const tzMode = profile.timezone?.mode ?? "Manual";
  const geoMode = profile.geolocation?.mode ?? "Manual";
  const screenMode = profile.screen?.mode ?? "Manual";
  const webglMode = profile.webgl?.mode ?? "Manual";
  const secChUaMode = profile.sec_ch_ua?.mode ?? "Manual";

  return (
    <div className={className}>
      <Tabs defaultValue="identity" className="w-full">
        <TabsList className="w-full">
          <TabsTrigger value="identity" className="flex-1 text-xs">
            {t("fingerprintProfile.identity")}
          </TabsTrigger>
          <TabsTrigger value="network" className="flex-1 text-xs">
            {t("fingerprintProfile.network")}
          </TabsTrigger>
          <TabsTrigger value="rendering" className="flex-1 text-xs">
            {t("fingerprintProfile.rendering")}
          </TabsTrigger>
          <TabsTrigger value="hardware" className="flex-1 text-xs">
            {t("fingerprintProfile.hardware")}
          </TabsTrigger>
        </TabsList>

        {/* Identity Tab */}
        <TabsContent value="identity" className="mt-4 space-y-4">
          <div className="space-y-3">
            <Label>{t("fingerprintProfile.userAgent")}</Label>
            <div className="flex items-center gap-2">
              <Select
                value={uaMode}
                onValueChange={(v: AutoOrManual) =>
                  updateField("user_agent", {
                    mode: v,
                    value: uaValue || undefined,
                  })
                }
                disabled={readOnly}
              >
                <SelectTrigger className="w-[120px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="Auto">
                    {t("fingerprintProfile.auto")}
                  </SelectItem>
                  <SelectItem value="Manual">
                    {t("fingerprintProfile.manual")}
                  </SelectItem>
                </SelectContent>
              </Select>
              {uaMode === "Manual" && (
                <Input
                  value={uaValue}
                  onChange={(e) =>
                    updateField("user_agent", {
                      mode: "Manual",
                      value: e.target.value,
                    })
                  }
                  placeholder={t("fingerprintProfile.uaPlaceholder")}
                  disabled={readOnly}
                  className="flex-1"
                />
              )}
            </div>
          </div>

          <div className="space-y-3">
            <Label>{t("fingerprintProfile.platform")}</Label>
            <Input
              value={profile.platform ?? ""}
              onChange={(e) =>
                updateField("platform", e.target.value || undefined)
              }
              placeholder="Win32"
              disabled={readOnly}
            />
          </div>

          <div className="space-y-3">
            <Label>{t("fingerprintProfile.language")}</Label>
            <div className="flex items-center gap-2">
              <Select
                value={langMode}
                onValueChange={(v: AutoOrManual) =>
                  updateField("language", {
                    mode: v,
                    language: profile.language?.language,
                    languages: profile.language?.languages,
                  })
                }
                disabled={readOnly}
              >
                <SelectTrigger className="w-[120px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="Auto">
                    {t("fingerprintProfile.auto")}
                  </SelectItem>
                  <SelectItem value="Manual">
                    {t("fingerprintProfile.manual")}
                  </SelectItem>
                </SelectContent>
              </Select>
              {langMode === "Manual" && (
                <Input
                  value={profile.language?.language ?? ""}
                  onChange={(e) =>
                    updateField("language", {
                      mode: "Manual",
                      language: e.target.value,
                      languages: [e.target.value],
                    })
                  }
                  placeholder="en-US"
                  disabled={readOnly}
                  className="flex-1"
                />
              )}
            </div>
          </div>

          <div className="space-y-3">
            <Label>{t("fingerprintProfile.timezone")}</Label>
            <div className="flex items-center gap-2">
              <Select
                value={tzMode}
                onValueChange={(v: AutoOrManual) =>
                  updateField("timezone", {
                    mode: v,
                    name: profile.timezone?.name,
                    offset: profile.timezone?.offset,
                  })
                }
                disabled={readOnly}
              >
                <SelectTrigger className="w-[120px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="Auto">
                    {t("fingerprintProfile.auto")}
                  </SelectItem>
                  <SelectItem value="Manual">
                    {t("fingerprintProfile.manual")}
                  </SelectItem>
                </SelectContent>
              </Select>
              {tzMode === "Manual" && (
                <>
                  <Input
                    value={profile.timezone?.name ?? ""}
                    onChange={(e) =>
                      updateField("timezone", {
                        mode: "Manual",
                        name: e.target.value,
                        offset: profile.timezone?.offset,
                      })
                    }
                    placeholder="America/New_York"
                    disabled={readOnly}
                    className="flex-1"
                  />
                  <Input
                    type="number"
                    value={profile.timezone?.offset ?? ""}
                    onChange={(e) =>
                      updateField("timezone", {
                        mode: "Manual",
                        name: profile.timezone?.name,
                        offset: e.target.value
                          ? Number(e.target.value)
                          : undefined,
                      })
                    }
                    placeholder={t("fingerprintProfile.offsetPlaceholder")}
                    disabled={readOnly}
                    className="w-[100px]"
                  />
                </>
              )}
            </div>
          </div>

          <div className="space-y-3">
            <Label>{t("fingerprintProfile.secChUa")}</Label>
            <Select
              value={secChUaMode}
              onValueChange={(v: AutoOrManual) =>
                updateField("sec_ch_ua", {
                  mode: v,
                  brands: profile.sec_ch_ua?.brands ?? [],
                })
              }
              disabled={readOnly}
            >
              <SelectTrigger className="w-[120px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="Auto">
                  {t("fingerprintProfile.auto")}
                </SelectItem>
                <SelectItem value="Manual">
                  {t("fingerprintProfile.manual")}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </TabsContent>

        {/* Network & Privacy Tab */}
        <TabsContent value="network" className="mt-4 space-y-4">
          <div className="space-y-3">
            <Label>{t("fingerprintProfile.webRTC")}</Label>
            <Select
              value={profile.webrtc?.mode ?? "Allow"}
              onValueChange={(v: WebRtcMode) =>
                updateField("webrtc", { mode: v })
              }
              disabled={readOnly}
            >
              <SelectTrigger className="w-[200px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="Allow">
                  {t("fingerprintProfile.webrtcAllow")}
                </SelectItem>
                <SelectItem value="Replace">
                  {t("fingerprintProfile.webrtcReplace")}
                </SelectItem>
                <SelectItem value="Block">
                  {t("fingerprintProfile.webrtcBlock")}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-3">
            <Label>{t("fingerprintProfile.geolocation")}</Label>
            <div className="flex items-center gap-2">
              <Select
                value={geoMode}
                onValueChange={(v: AutoOrManual) =>
                  updateField("geolocation", {
                    mode: v,
                    longitude: profile.geolocation?.longitude,
                    latitude: profile.geolocation?.latitude,
                    precision: profile.geolocation?.precision,
                    permission: profile.geolocation?.permission ?? "Ask",
                  })
                }
                disabled={readOnly}
              >
                <SelectTrigger className="w-[120px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="Auto">
                    {t("fingerprintProfile.auto")}
                  </SelectItem>
                  <SelectItem value="Manual">
                    {t("fingerprintProfile.manual")}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            {geoMode === "Manual" && (
              <div className="grid grid-cols-3 gap-2">
                <div>
                  <Label className="text-xs">
                    {t("fingerprintProfile.latitude")}
                  </Label>
                  <Input
                    type="number"
                    step="0.0001"
                    value={profile.geolocation?.latitude ?? ""}
                    onChange={(e) =>
                      updateField("geolocation", {
                        ...profile.geolocation,
                        mode: "Manual",
                        latitude: e.target.value
                          ? Number(e.target.value)
                          : undefined,
                        permission: profile.geolocation?.permission ?? "Ask",
                      })
                    }
                    disabled={readOnly}
                  />
                </div>
                <div>
                  <Label className="text-xs">
                    {t("fingerprintProfile.longitude")}
                  </Label>
                  <Input
                    type="number"
                    step="0.0001"
                    value={profile.geolocation?.longitude ?? ""}
                    onChange={(e) =>
                      updateField("geolocation", {
                        ...profile.geolocation,
                        mode: "Manual",
                        longitude: e.target.value
                          ? Number(e.target.value)
                          : undefined,
                        permission: profile.geolocation?.permission ?? "Ask",
                      })
                    }
                    disabled={readOnly}
                  />
                </div>
                <div>
                  <Label className="text-xs">
                    {t("fingerprintProfile.permission")}
                  </Label>
                  <Select
                    value={profile.geolocation?.permission ?? "Ask"}
                    onValueChange={(v: GeoPermission) =>
                      updateField("geolocation", {
                        ...profile.geolocation,
                        mode: "Manual",
                        permission: v,
                      })
                    }
                    disabled={readOnly}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="Ask">
                        {t("fingerprintProfile.geoAsk")}
                      </SelectItem>
                      <SelectItem value="Allow">
                        {t("fingerprintProfile.geoAllow")}
                      </SelectItem>
                      <SelectItem value="Block">
                        {t("fingerprintProfile.geoBlock")}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
            )}
          </div>

          <div className="flex items-center justify-between">
            <div>
              <Label>{t("fingerprintProfile.dnt")}</Label>
              <p className="text-xs text-muted-foreground">
                {t("fingerprintProfile.dntDescription")}
              </p>
            </div>
            <Switch
              checked={profile.dnt ?? false}
              onCheckedChange={(v) => updateField("dnt", v)}
              disabled={readOnly}
            />
          </div>

          <div className="space-y-3">
            <Label>{t("fingerprintProfile.ssl")}</Label>
            <div className="flex items-center gap-2">
              <Switch
                checked={profile.ssl?.enabled ?? false}
                onCheckedChange={(v) =>
                  updateField("ssl", {
                    enabled: v,
                    disabled_versions: profile.ssl?.disabled_versions ?? [],
                  })
                }
                disabled={readOnly}
              />
              <span className="text-sm text-muted-foreground">
                {t("fingerprintProfile.sslEnabled")}
              </span>
            </div>
          </div>
        </TabsContent>

        {/* Rendering Tab */}
        <TabsContent value="rendering" className="mt-4 space-y-4">
          <NoiseModeField
            label={t("fingerprintProfile.canvas")}
            description={t("fingerprintProfile.canvasDescription")}
            value={profile.canvas ?? "Default"}
            onChange={(v: NoiseMode) => updateField("canvas", v)}
            readOnly={readOnly}
          />
          <NoiseModeField
            label={t("fingerprintProfile.audioContext")}
            description={t("fingerprintProfile.audioContextDescription")}
            value={profile.audio_context ?? "Default"}
            onChange={(v: NoiseMode) => updateField("audio_context", v)}
            readOnly={readOnly}
          />
          <NoiseModeField
            label={t("fingerprintProfile.clientRects")}
            description={t("fingerprintProfile.clientRectsDescription")}
            value={profile.client_rects ?? "Default"}
            onChange={(v: NoiseMode) => updateField("client_rects", v)}
            readOnly={readOnly}
          />
          <NoiseModeField
            label={t("fingerprintProfile.speechVoices")}
            description={t("fingerprintProfile.speechVoicesDescription")}
            value={profile.speech_voices ?? "Default"}
            onChange={(v: NoiseMode) => updateField("speech_voices", v)}
            readOnly={readOnly}
          />

          <div className="space-y-3">
            <Label>{t("fingerprintProfile.webgl")}</Label>
            <div className="flex items-center gap-2">
              <Select
                value={webglMode}
                onValueChange={(v: AutoOrManual) =>
                  updateField("webgl", {
                    mode: v,
                    vendor: profile.webgl?.vendor,
                    renderer: profile.webgl?.renderer,
                  })
                }
                disabled={readOnly}
              >
                <SelectTrigger className="w-[120px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="Auto">
                    {t("fingerprintProfile.auto")}
                  </SelectItem>
                  <SelectItem value="Manual">
                    {t("fingerprintProfile.manual")}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            {webglMode === "Manual" && (
              <div className="grid grid-cols-2 gap-2">
                <div>
                  <Label className="text-xs">
                    {t("fingerprintProfile.webglVendor")}
                  </Label>
                  <Input
                    value={profile.webgl?.vendor ?? ""}
                    onChange={(e) =>
                      updateField("webgl", {
                        mode: "Manual",
                        vendor: e.target.value,
                        renderer: profile.webgl?.renderer,
                      })
                    }
                    disabled={readOnly}
                  />
                </div>
                <div>
                  <Label className="text-xs">
                    {t("fingerprintProfile.webglRenderer")}
                  </Label>
                  <Input
                    value={profile.webgl?.renderer ?? ""}
                    onChange={(e) =>
                      updateField("webgl", {
                        mode: "Manual",
                        vendor: profile.webgl?.vendor,
                        renderer: e.target.value,
                      })
                    }
                    disabled={readOnly}
                  />
                </div>
              </div>
            )}
          </div>
        </TabsContent>

        {/* Hardware Tab */}
        <TabsContent value="hardware" className="mt-4 space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>{t("fingerprintProfile.cpu")}</Label>
              <Input
                type="number"
                min={1}
                max={128}
                value={profile.cpu ?? ""}
                onChange={(e) =>
                  updateField(
                    "cpu",
                    e.target.value ? Number(e.target.value) : undefined,
                  )
                }
                placeholder="8"
                disabled={readOnly}
              />
            </div>
            <div className="space-y-2">
              <Label>{t("fingerprintProfile.memory")}</Label>
              <Input
                type="number"
                min={1}
                max={128}
                value={profile.memory ?? ""}
                onChange={(e) =>
                  updateField(
                    "memory",
                    e.target.value ? Number(e.target.value) : undefined,
                  )
                }
                placeholder="8"
                disabled={readOnly}
              />
            </div>
          </div>

          <div className="space-y-3">
            <Label>{t("fingerprintProfile.screen")}</Label>
            <div className="flex items-center gap-2">
              <Select
                value={screenMode}
                onValueChange={(v: AutoOrManual) =>
                  updateField("screen", {
                    mode: v,
                    width: profile.screen?.width,
                    height: profile.screen?.height,
                  })
                }
                disabled={readOnly}
              >
                <SelectTrigger className="w-[120px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="Auto">
                    {t("fingerprintProfile.auto")}
                  </SelectItem>
                  <SelectItem value="Manual">
                    {t("fingerprintProfile.manual")}
                  </SelectItem>
                </SelectContent>
              </Select>
              {screenMode === "Manual" && (
                <>
                  <Input
                    type="number"
                    value={profile.screen?.width ?? ""}
                    onChange={(e) =>
                      updateField("screen", {
                        mode: "Manual",
                        width: e.target.value
                          ? Number(e.target.value)
                          : undefined,
                        height: profile.screen?.height,
                      })
                    }
                    placeholder="1920"
                    disabled={readOnly}
                    className="w-[100px]"
                  />
                  <span className="text-muted-foreground">x</span>
                  <Input
                    type="number"
                    value={profile.screen?.height ?? ""}
                    onChange={(e) =>
                      updateField("screen", {
                        mode: "Manual",
                        width: profile.screen?.width,
                        height: e.target.value
                          ? Number(e.target.value)
                          : undefined,
                      })
                    }
                    placeholder="1080"
                    disabled={readOnly}
                    className="w-[100px]"
                  />
                </>
              )}
            </div>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <Label>{t("fingerprintProfile.gpuAcceleration")}</Label>
              <p className="text-xs text-muted-foreground">
                {t("fingerprintProfile.gpuAccelerationDescription")}
              </p>
            </div>
            <Switch
              checked={profile.gpu_acceleration !== false}
              onCheckedChange={(v) => updateField("gpu_acceleration", v)}
              disabled={readOnly}
            />
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}

function NoiseModeField({
  label,
  description,
  value,
  onChange,
  readOnly,
}: {
  label: string;
  description: string;
  value: NoiseMode;
  onChange: (v: NoiseMode) => void;
  readOnly: boolean;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex items-center justify-between">
      <div>
        <Label>{label}</Label>
        <p className="text-xs text-muted-foreground">{description}</p>
      </div>
      <Select value={value} onValueChange={onChange} disabled={readOnly}>
        <SelectTrigger className="w-[120px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="Default">
            {t("fingerprintProfile.noiseDefault")}
          </SelectItem>
          <SelectItem value="Random">
            {t("fingerprintProfile.noiseRandom")}
          </SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
}

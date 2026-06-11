"use client";

import * as React from "react";
import { useTranslation } from "react-i18next";
import { FiWifi } from "react-icons/fi";
import { LuPanelTop, LuPlay, LuSquare } from "react-icons/lu";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { translateBackendError } from "@/lib/backend-errors";
import type {
  BatchLaunchOptions,
  BatchTaskEvent,
  FailurePolicy,
  PostLaunchAction,
  ProfileProxyDiagnosticResult,
  WindowLayoutCapabilities,
  WindowLayoutMode,
  WindowLayoutOptions,
} from "@/types";

interface BatchOperationsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  selectedCount: number;
  activeTaskEvents: BatchTaskEvent[];
  diagnosticResults: ProfileProxyDiagnosticResult[];
  isRunning: boolean;
  capabilities?: WindowLayoutCapabilities;
  onLaunch: (options: BatchLaunchOptions) => Promise<void>;
  onStop: () => Promise<void>;
  onArrange: (options: WindowLayoutOptions) => Promise<void>;
  onDiagnose: () => Promise<void>;
}

export function BatchOperationsDialog({
  open,
  onOpenChange,
  selectedCount,
  activeTaskEvents,
  diagnosticResults,
  isRunning,
  capabilities,
  onLaunch,
  onStop,
  onArrange,
  onDiagnose,
}: BatchOperationsDialogProps) {
  const { t } = useTranslation();
  const [concurrency, setConcurrency] = React.useState(3);
  const [launchIntervalSeconds, setLaunchIntervalSeconds] = React.useState(1);
  const [failurePolicy, setFailurePolicy] =
    React.useState<FailurePolicy>("continue");
  const [postLaunchAction, setPostLaunchAction] =
    React.useState<PostLaunchAction>("none");
  const [layoutMode, setLayoutMode] = React.useState<WindowLayoutMode>("grid");
  const [gap, setGap] = React.useState(8);
  const [preserveAspectRatio, setPreserveAspectRatio] = React.useState(false);

  const launchOptions = React.useMemo<BatchLaunchOptions>(
    () => ({
      concurrency,
      launch_interval_ms: launchIntervalSeconds * 1000,
      failure_policy: failurePolicy,
      post_launch_action: postLaunchAction,
    }),
    [concurrency, failurePolicy, launchIntervalSeconds, postLaunchAction],
  );

  const layoutOptions = React.useMemo<WindowLayoutOptions>(
    () => ({
      mode: layoutMode,
      gap,
      preserve_aspect_ratio: preserveAspectRatio,
    }),
    [gap, layoutMode, preserveAspectRatio],
  );

  const arrangeDisabled =
    isRunning || selectedCount === 0 || capabilities?.supported === false;
  const hasResults =
    activeTaskEvents.length > 0 || diagnosticResults.length > 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{t("batchOperations.title")}</DialogTitle>
          <DialogDescription>
            {t("batchOperations.description")}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="rounded-md border p-3 text-sm">
            {t("batchOperations.targetSelected")}: {selectedCount}
          </div>

          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            <div className="space-y-1 text-sm">
              <span>{t("batchOperations.concurrency")}</span>
              <Input
                aria-label={t("batchOperations.concurrency")}
                type="number"
                min={1}
                max={10}
                value={concurrency}
                onChange={(event) =>
                  setConcurrency(Number.parseInt(event.target.value, 10) || 1)
                }
              />
            </div>
            <div className="space-y-1 text-sm">
              <span>{t("batchOperations.launchInterval")}</span>
              <Input
                aria-label={t("batchOperations.launchInterval")}
                type="number"
                min={0}
                max={30}
                value={launchIntervalSeconds}
                onChange={(event) =>
                  setLaunchIntervalSeconds(
                    Number.parseInt(event.target.value, 10) || 0,
                  )
                }
              />
            </div>
            <div className="space-y-1 text-sm">
              <span>{t("batchOperations.failurePolicy")}</span>
              <Select
                value={failurePolicy}
                onValueChange={(value) =>
                  setFailurePolicy(value as FailurePolicy)
                }
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="continue">
                    {t("batchOperations.continueOnError")}
                  </SelectItem>
                  <SelectItem value="stop_on_first_error">
                    {t("batchOperations.stopOnFirstError")}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1 text-sm">
              <span>{t("batchOperations.postLaunchAction")}</span>
              <Select
                value={postLaunchAction}
                onValueChange={(value) =>
                  setPostLaunchAction(value as PostLaunchAction)
                }
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">
                    {t("batchOperations.postLaunchNone")}
                  </SelectItem>
                  <SelectItem value="arrange_windows">
                    {t("batchOperations.postLaunchArrange")}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1 text-sm">
              <span>{t("batchOperations.layoutMode")}</span>
              <Select
                value={layoutMode}
                onValueChange={(value) =>
                  setLayoutMode(value as WindowLayoutMode)
                }
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="grid">
                    {t("batchOperations.layoutGrid")}
                  </SelectItem>
                  <SelectItem value="horizontal">
                    {t("batchOperations.layoutHorizontal")}
                  </SelectItem>
                  <SelectItem value="vertical">
                    {t("batchOperations.layoutVertical")}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1 text-sm">
              <span>{t("batchOperations.gap")}</span>
              <Input
                aria-label={t("batchOperations.gap")}
                type="number"
                min={0}
                max={64}
                value={gap}
                onChange={(event) =>
                  setGap(Number.parseInt(event.target.value, 10) || 0)
                }
              />
            </div>
          </div>

          <div className="flex items-center gap-2 text-sm">
            <Checkbox
              aria-label={t("batchOperations.preserveAspectRatio")}
              checked={preserveAspectRatio}
              onCheckedChange={(checked) =>
                setPreserveAspectRatio(checked === true)
              }
            />
            {t("batchOperations.preserveAspectRatio")}
          </div>

          <div className="flex flex-wrap gap-2">
            <Button
              onClick={() => void onLaunch(launchOptions)}
              disabled={isRunning || selectedCount === 0}
            >
              <LuPlay className="size-4" />
              {t("batchOperations.launchSelected")}
            </Button>
            <Button
              variant="secondary"
              onClick={() => void onStop()}
              disabled={isRunning || selectedCount === 0}
            >
              <LuSquare className="size-4" />
              {t("batchOperations.stopSelected")}
            </Button>
            <Button
              variant="secondary"
              onClick={() => void onArrange(layoutOptions)}
              disabled={arrangeDisabled}
            >
              <LuPanelTop className="size-4" />
              {t("batchOperations.arrangeWindows")}
            </Button>
            <Button
              variant="secondary"
              onClick={() => void onDiagnose()}
              disabled={isRunning || selectedCount === 0}
            >
              <FiWifi className="size-4" />
              {t("batchOperations.diagnoseProxies")}
            </Button>
          </div>

          <div className="space-y-2">
            <h3 className="text-sm font-medium">
              {t("batchOperations.results")}
            </h3>
            {hasResults ? (
              <div className="max-h-72 overflow-auto rounded-md border">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{t("batchOperations.profile")}</TableHead>
                      <TableHead>{t("batchOperations.result")}</TableHead>
                      <TableHead>{t("batchOperations.exitIp")}</TableHead>
                      <TableHead>{t("batchOperations.country")}</TableHead>
                      <TableHead>{t("batchOperations.latency")}</TableHead>
                      <TableHead>{t("batchOperations.source")}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {activeTaskEvents.map((event) => (
                      <TableRow key={`${event.task_id}-${event.profile_id}`}>
                        <TableCell>{event.profile_name}</TableCell>
                        <TableCell>
                          {event.error
                            ? translateBackendError(t, event.error)
                            : event.status}
                        </TableCell>
                        <TableCell />
                        <TableCell />
                        <TableCell />
                        <TableCell />
                      </TableRow>
                    ))}
                    {diagnosticResults.map((result) => (
                      <TableRow key={result.profile_id}>
                        <TableCell>{result.profile_name}</TableCell>
                        <TableCell>
                          {result.error
                            ? translateBackendError(t, result.error)
                            : result.is_valid
                              ? t("batchOperations.valid")
                              : t("batchOperations.failed")}
                        </TableCell>
                        <TableCell>{result.ip ?? ""}</TableCell>
                        <TableCell>{result.country ?? ""}</TableCell>
                        <TableCell>
                          {result.latency_ms != null
                            ? `${result.latency_ms}ms`
                            : ""}
                        </TableCell>
                        <TableCell>{result.source ?? ""}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            ) : (
              <div className="rounded-md border p-4 text-sm text-muted-foreground">
                {t("batchOperations.noResults")}
              </div>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

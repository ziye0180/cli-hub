import { BarChart3, Check, Copy, Edit, Play, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

interface ProviderActionsProps {
  isCurrent: boolean;
  onSwitch: () => void;
  onEdit: () => void;
  onDuplicate: () => void;
  onConfigureUsage: () => void;
  onDelete: () => void;
}

export function ProviderActions({
  isCurrent,
  onSwitch,
  onEdit,
  onDuplicate,
  onConfigureUsage,
  onDelete,
}: ProviderActionsProps) {
  const { t } = useTranslation();
  const iconButtonClass = "h-8 w-8 p-1";

  return (
    <div className="flex items-center gap-1.5">
      <Button
        size="sm"
        variant={isCurrent ? "secondary" : "default"}
        onClick={onSwitch}
        disabled={isCurrent}
        className={cn(
          "w-[4.5rem] px-2.5",
          isCurrent &&
            "bg-gray-200 text-muted-foreground hover:bg-gray-200 hover:text-muted-foreground dark:bg-gray-700 dark:hover:bg-gray-700",
        )}
      >
        {isCurrent ? (
          <>
            <Check className="h-4 w-4" />
            {t("provider.inUse")}
          </>
        ) : (
          <>
            <Play className="h-4 w-4" />
            {t("provider.enable")}
          </>
        )}
      </Button>

      <div className="flex items-center gap-1">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              size="icon"
              variant="ghost"
              onClick={onEdit}
              className={iconButtonClass}
            >
              <Edit className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{t("common.edit")}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              size="icon"
              variant="ghost"
              onClick={onDuplicate}
              className={iconButtonClass}
            >
              <Copy className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{t("provider.duplicate")}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              size="icon"
              variant="ghost"
              onClick={onConfigureUsage}
              className={iconButtonClass}
            >
              <BarChart3 className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{t("provider.configureUsage")}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              size="icon"
              variant="ghost"
              onClick={isCurrent ? undefined : onDelete}
              className={cn(
                iconButtonClass,
                !isCurrent && "hover:text-red-500 dark:hover:text-red-400",
                isCurrent &&
                  "opacity-40 cursor-not-allowed text-muted-foreground",
              )}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{t("common.delete")}</TooltipContent>
        </Tooltip>
      </div>
    </div>
  );
}

import { Users } from "lucide-react";
import { useTranslation } from "react-i18next";

export function ProviderEmptyState() {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-muted-foreground/30 p-10 text-center">
      <div className="mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-muted">
        <Users className="h-7 w-7 text-muted-foreground" />
      </div>
      <h3 className="text-lg font-semibold">{t("provider.noProviders")}</h3>
      <p className="mt-2 max-w-sm text-sm text-muted-foreground">
        {t("provider.noProvidersDescription")}
      </p>
    </div>
  );
}

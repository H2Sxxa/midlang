import { TranslateIcon } from "@phosphor-icons/react";
import { createFileRoute, redirect, useNavigate } from "@tanstack/react-router";
import type { ToPathOption } from "@tanstack/react-router";
import { useEffect, useState, type SubmitEvent } from "react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAuth } from "@/hooks/use-auth";
import { APP_TITLE } from "@/lib/navigation";

type LoginSearch = {
  redirect?: string;
};

type LoginFormErrors = {
  remoteUrl?: string;
  token?: string;
};

function parseLoginSearch(search: Record<string, unknown>): LoginSearch {
  const redirectTarget = search.redirect;

  return typeof redirectTarget === "string" ? { redirect: redirectTarget } : {};
}

function isHttpUrl(value: string): boolean {
  if (!URL.canParse(value)) {
    return false;
  }

  const { protocol } = new URL(value);
  return protocol === "http:" || protocol === "https:";
}

/**
 * The redirect target comes from the URL, so it can only be checked at runtime:
 * only same-app absolute paths are accepted, which rules out open redirects.
 */
function resolveRedirectTarget(
  redirectTarget: string | undefined,
): ToPathOption {
  if (
    redirectTarget === undefined ||
    !redirectTarget.startsWith("/") ||
    redirectTarget.startsWith("//")
  ) {
    return "/";
  }

  return redirectTarget as ToPathOption;
}

function validateLoginForm(remoteUrl: string, token: string): LoginFormErrors {
  const errors: LoginFormErrors = {};

  if (remoteUrl === "") {
    errors.remoteUrl = "Remote URL is required.";
  } else if (!isHttpUrl(remoteUrl)) {
    errors.remoteUrl = "Remote URL must be an http(s) URL.";
  }

  if (token === "") {
    errors.token = "API token is required.";
  }

  return errors;
}

export const Route = createFileRoute("/login/")({
  validateSearch: parseLoginSearch,
  beforeLoad: ({ context, search }) => {
    if (context.auth?.status === "authenticated") {
      throw redirect({
        to: resolveRedirectTarget(search.redirect),
        replace: true,
      });
    }
  },
  component: LoginPage,
});

function LoginPage() {
  const auth = useAuth();
  const navigate = useNavigate();
  const search = Route.useSearch();
  const [remoteUrl, setRemoteUrl] = useState(auth.remoteUrl);
  const [token, setToken] = useState("");
  const [formErrors, setFormErrors] = useState<LoginFormErrors>({});
  const [submitError, setSubmitError] = useState<string | null>(null);
  const target = resolveRedirectTarget(search.redirect);
  const isSubmitting = auth.status === "authenticating";

  // Runs after the provider committed the new state, so the router context used by
  // the `_app` guard is already authenticated.
  useEffect(() => {
    if (auth.status !== "authenticated") {
      return;
    }

    void navigate({ to: target, replace: true });
  }, [auth.status, navigate, target]);

  const handleSubmit = async (event: SubmitEvent) => {
    event.preventDefault();

    const trimmedRemoteUrl = remoteUrl.trim();
    const trimmedToken = token.trim();
    const validationErrors = validateLoginForm(trimmedRemoteUrl, trimmedToken);

    setFormErrors(validationErrors);

    if (Object.keys(validationErrors).length > 0) {
      return;
    }

    setSubmitError(null);

    try {
      await auth.login(trimmedToken, trimmedRemoteUrl);
    } catch (error) {
      setSubmitError(
        `Could not sign in: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  };

  return (
    <div className="flex min-h-dvh items-center justify-center p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <div className="flex size-8 items-center justify-center bg-sidebar-primary text-sidebar-primary-foreground">
            <TranslateIcon />
          </div>
          <CardTitle>Sign in to {APP_TITLE}</CardTitle>
          <CardDescription>
            Connect to a MidLang server to manage its translation resources.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form
            className="flex flex-col gap-3"
            noValidate
            onSubmit={handleSubmit}
          >
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="remote-url">Remote URL</Label>
              <Input
                id="remote-url"
                name="remoteUrl"
                autoComplete="off"
                spellCheck={false}
                placeholder="http://localhost:4321"
                value={remoteUrl}
                aria-invalid={formErrors.remoteUrl !== undefined}
                onChange={(event) => setRemoteUrl(event.target.value)}
              />
              {formErrors.remoteUrl !== undefined && (
                <p className="text-destructive text-xs">
                  {formErrors.remoteUrl}
                </p>
              )}
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="token">API token</Label>
              <Input
                id="token"
                name="token"
                type="password"
                autoComplete="off"
                spellCheck={false}
                placeholder="Token issued by the server"
                value={token}
                aria-invalid={formErrors.token !== undefined}
                onChange={(event) => setToken(event.target.value)}
              />
              {formErrors.token !== undefined && (
                <p className="text-destructive text-xs">{formErrors.token}</p>
              )}
            </div>
            {submitError !== null && (
              <p className="text-destructive text-xs" role="alert">
                {submitError}
              </p>
            )}
            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? "Signing in…" : "Sign in"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

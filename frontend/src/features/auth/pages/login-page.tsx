import { ShieldCheck } from "lucide-react";
import { LoginForm } from "@/features/auth/components/login-form";

export function LoginPage() {
  return (
    <div className="mx-auto flex min-h-[70vh] w-full max-w-md items-center justify-center">
      <section className="w-full space-y-7 rounded-2xl border border-border bg-card p-7 shadow-xl shadow-black/10 sm:p-9">
        <div className="space-y-4">
          <div className="flex size-12 items-center justify-center rounded-xl border border-primary/20 bg-primary/10 text-primary">
            <ShieldCheck size={23} />
          </div>
          <div>
            <p className="text-xs font-medium uppercase tracking-[0.2em] text-primary">
              Ravenry / Workspace
            </p>
            <h1 className="mt-2 text-2xl font-semibold tracking-tight">
              Welcome back
            </h1>
            <p className="mt-2 text-sm text-muted-foreground">
              Sign in to manage your workspace.
            </p>
          </div>
        </div>
        <LoginForm />
      </section>
    </div>
  );
}

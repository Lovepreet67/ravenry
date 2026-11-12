import { KeyRound, ShieldCheck } from "lucide-react";
import { AdminLoginForm } from "@/features/auth/components/admin-login-form";

export function AdminLoginPage() {
  return (
    <div className="mx-auto flex min-h-[70vh] w-full max-w-md items-center justify-center">
      <section className="w-full space-y-7 rounded-2xl border border-border bg-card p-7 shadow-xl shadow-black/10 sm:p-9">
        <div className="space-y-4">
          <div className="flex size-12 items-center justify-center rounded-xl border border-primary/20 bg-primary/10 text-primary">
            <KeyRound size={23} />
          </div>
          <div>
            <p className="flex items-center gap-2 text-xs font-medium uppercase tracking-[0.2em] text-primary">
              <ShieldCheck size={14} />
              Ravenry / Administrator
            </p>
            <h1 className="mt-2 text-2xl font-semibold tracking-tight">
              Admin access
            </h1>
            <p className="mt-2 text-sm text-muted-foreground">
              Verify your administrator key to continue.
            </p>
          </div>
        </div>
        <AdminLoginForm />
      </section>
    </div>
  );
}

export default function access(state: { admin?: { role?: string } | null; user?: unknown } | undefined) {
  return { signedIn: Boolean(state?.user), administrator: Boolean(state?.admin), superAdministrator: state?.admin?.role === 'super_admin' }
}

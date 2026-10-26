import { ConsumerTheme } from '../components/consumer-theme'
import { request } from '../services/request'
import { Action, Field, SettingsForm } from '../components/controls'
import { UserDownloads } from '../downloads'
import { useConfirmDialog } from '../confirm-dialog'
import { byteLabel } from '../formatting'
import { OperationToast } from '../operation-toast'
import { PasswordAccess, UserPasswordChange } from '../local-accounts'
import * as React from 'react'
import multiavatar from '@multiavatar/multiavatar/esm'
import { ArrowLeft, Languages, Mail, Moon, LogIn, LogOut, ServerCog, Sun, UserRound } from 'lucide-react'
import { readStoredPreference } from '../preferences'
import { type Locale, type Theme, type UserProfile, type UserUsage, type PublicAuthProvider, type LinkedIdentity, type PublicConfig } from '../types'
import { accountMessages } from '../i18n/messages'
import { HttpSecurityBanner, ConsoleMetric } from '../components/shared'

export function UserPage() {
  return <ConsumerTheme><UserPageContent /></ConsumerTheme>
}

function UserPageContent() {
  const confirmAction = useConfirmDialog()
  const [locale, setLocale] = React.useState<Locale>(() => readStoredPreference(localStorage, 'mirrorproxy.locale', 'en', ['en', 'zh']))
  const [theme, setTheme] = React.useState<Theme>(() => readStoredPreference(localStorage, 'mirrorproxy.theme', 'light', ['light', 'dark']))
  const [email, setEmail] = React.useState(() => new URLSearchParams(location.search).get('email') ?? '')
  const [code, setCode] = React.useState('')
  const [profile, setProfile] = React.useState<UserProfile | null>(null)
  const [usage, setUsage] = React.useState<UserUsage | null>(null)
  const [message, setMessage] = React.useState('')
  const [providers, setProviders] = React.useState<PublicAuthProvider[]>([])
  const [identities, setIdentities] = React.useState<LinkedIdentity[]>([])
  const [registration, setRegistration] = React.useState<NonNullable<PublicConfig['registration']> | null>(null)
  const [sending, setSending] = React.useState(false)
  const [verifying, setVerifying] = React.useState(false)
  const invitation = new URLSearchParams(location.search).get('invitation')
  const [registrationToken, setRegistrationToken] = React.useState(() => new URLSearchParams(location.hash.slice(1)).get('register'))
  React.useEffect(() => {
    const readInvitation = () => setRegistrationToken(new URLSearchParams(window.location.hash.slice(1)).get('register'))
    window.addEventListener('hashchange', readInvitation)
    return () => window.removeEventListener('hashchange', readInvitation)
  }, [])
  const magicToken = new URLSearchParams(location.search).get('token')
  const oauthError = new URLSearchParams(location.search).get('oauth_error')
  const automaticToken = magicToken
  const t = accountMessages[locale]
  const oauthErrorMessage = oauthError === 'registration_disabled' || oauthError === 'invitation_required' || oauthError === 'invalid_invitation'
    ? t.oauthRegistrationClosed
    : oauthError === 'verified_email_required'
      ? t.oauthVerifiedEmail
      : oauthError === 'manual_link_required'
        ? t.oauthManualLink
        : oauthError === 'provider_denied'
          ? t.oauthDenied
          : oauthError
            ? t.oauthFailed
            : ''
  const feedback = message || oauthErrorMessage
  const feedbackSuccess = feedback === t.emailSent || feedback.endsWith(t.disconnected) || feedback === '密码已修改，请重新登录。' || feedback === 'Password changed. Please sign in again.'
  const profileAvatarUrl = profile
    ? `data:image/svg+xml;charset=UTF-8,${encodeURIComponent(multiavatar(profile.user.username ?? profile.user.email))}`
    : null

  React.useEffect(() => {
    document.documentElement.dataset.theme = theme
    localStorage.setItem('mirrorproxy.theme', theme)
  }, [theme])
  React.useEffect(() => { localStorage.setItem('mirrorproxy.locale', locale); document.documentElement.dataset.locale = locale; document.documentElement.lang = locale === 'zh' ? 'zh-CN' : 'en' }, [locale])

  const loadProfile = React.useCallback(async () => {
    const [profileResponse, usageResponse, identitiesResponse] = await Promise.all([request('/api/account/profile'), request('/api/account/usage'), request('/api/account/providers')])
    if (profileResponse.ok) setProfile(await profileResponse.json() as UserProfile)
    if (usageResponse.ok) setUsage(await usageResponse.json() as UserUsage)
    if (identitiesResponse.ok) setIdentities(await identitiesResponse.json() as LinkedIdentity[])
  }, [])

  React.useEffect(() => { loadProfile().catch(() => undefined) }, [loadProfile])
  React.useEffect(() => {
    request('/api/auth/session').catch(() => undefined)
  }, [])
  React.useEffect(() => {
    Promise.all([
      request('/api/public-config').then((response) => response.ok ? response.json() as Promise<PublicConfig> : Promise.reject()),
      request('/api/auth/providers').then((response) => response.ok ? response.json() as Promise<PublicAuthProvider[]> : []),
    ]).then(([config, configuredProviders]) => {
      setRegistration(config.registration ?? { mode: 'disabled', allowed_email_domains: [], email_login_enabled: false })
      setProviders(Array.isArray(configuredProviders) ? configuredProviders : [])
    }).catch(() => setRegistration({ mode: 'disabled', allowed_email_domains: [], email_login_enabled: false }))
  }, [])
  React.useEffect(() => {
    if (!automaticToken || !email) return
    setVerifying(true)
    request('/api/auth/email/verify', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email, token: automaticToken }) })
      .then((response) => response.ok ? loadProfile() : Promise.reject())
      .then(() => window.history.replaceState({}, '', '/account'))
      .catch(() => setMessage(t.invalidLink))
      .finally(() => setVerifying(false))
  }, [automaticToken, email, loadProfile, t.invalidLink])

  const requestLogin = async (event: React.FormEvent) => {
    event.preventDefault()
    setSending(true); setMessage('')
    try {
      const response = await request('/api/auth/email/request', { method: 'POST', headers: { 'content-type': 'application/json', 'x-mirrorproxy-locale': locale }, body: JSON.stringify({ email, invitation_token: invitation }) })
      setMessage(response.ok ? t.emailSent : t.emailUnavailable)
    } finally { setSending(false) }
  }
  const verify = async (event: React.FormEvent) => {
    event.preventDefault()
    setVerifying(true); setMessage('')
    try {
      const response = await request('/api/auth/email/verify', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email, code }) })
      if (!response.ok) { setMessage(t.invalidCode); return }
      await loadProfile()
    } finally { setVerifying(false) }
  }
  const rotate = async () => {
    if (!await confirmAction({ locale, title: locale === 'zh' ? '更换专属代理地址' : 'Rotate routing address', message: t.confirmRotate, confirmLabel: locale === 'zh' ? '确认更换' : 'Rotate address', tone: 'danger' })) return
    const response = await request('/api/account/routing-id/rotate', { method: 'POST' })
    if (!response.ok) { setMessage(t.rotateFailed); return }
    await loadProfile()
  }
  const unlink = async (identity: LinkedIdentity) => {
    if (!await confirmAction({ locale, title: locale === 'zh' ? '解绑登录方式' : 'Disconnect sign-in method', message: `${t.confirmDisconnect}\n\n${identity.provider_name}`, confirmLabel: locale === 'zh' ? '确认解绑' : 'Disconnect', tone: 'danger' })) return
    const response = await request(`/api/account/providers/${identity.id}`, { method: 'DELETE' })
    setMessage(response.ok ? `${identity.provider_name} ${t.disconnected}` : t.disconnectFailed)
    if (response.ok) await loadProfile()
  }
  const providerUrl = (provider: PublicAuthProvider, link = false) => {
    const base = link ? `/api/account/providers/${encodeURIComponent(provider.slug)}/link/start` : `/api/auth/${encodeURIComponent(provider.slug)}/start`
    return !link && invitation ? `${base}?invitation=${encodeURIComponent(invitation)}` : base
  }

  const signOut = async () => {
    if (!await confirmAction({ locale, title: locale === 'zh' ? '退出账户' : 'Sign out', message: t.confirmSignOut, confirmLabel: locale === 'zh' ? '退出登录' : 'Sign out' })) return
    await request('/api/auth/logout', { method: 'POST' }).catch(() => undefined)
    setProfile(null); setUsage(null); setIdentities([])
  }

  const policy = (() => {
    if (registrationToken) return { tone: 'invite', title: locale === 'zh' ? '邀请注册' : 'Invitation registration', body: locale === 'zh' ? '填写账号密码完成注册，无需邮件验证。' : 'Choose an account and password to register. No email verification is needed.' }
    if (!registration) return null
    if (registration.mode === 'open') return { tone: 'open', title: t.openTitle, body: t.openBody }
    if (registration.mode === 'domain_allowlist') return { tone: 'domain', title: t.domainTitle, body: t.domainBody }
    if (registration.mode === 'invite_only' && (magicToken || invitation)) return { tone: 'invite', title: t.invitedTitle, body: t.invitedBody }
    if (registration.mode === 'invite_only') return { tone: 'closed', title: t.inviteTitle, body: t.inviteBody }
    return { tone: 'closed', title: t.disabledTitle, body: t.disabledBody }
  })()
  const registrationAvailable = registration?.mode === 'open' || registration?.mode === 'domain_allowlist' || (registration?.mode === 'invite_only' && Boolean(magicToken || invitation))

  return <main className={profile ? 'account-page consumer-account' : 'account-page consumer-auth'}><header className="account-topbar"><a className="brand-mark" href="/"><ServerCog size={18} /> {t.account}</a><div className="toolbar"><a className="account-back" href="/"><ArrowLeft size={16} /> {t.back}</a><Action className="icon-button" onClick={() => setLocale(locale === 'en' ? 'zh' : 'en')} title={t.language}><Languages size={18} /></Action><Action className="icon-button" onClick={() => setTheme(theme === 'light' ? 'dark' : 'light')} title={t.theme}>{theme === 'light' ? <Moon size={18} /> : <Sun size={18} />}</Action></div></header><HttpSecurityBanner locale={locale} /><section className="account-shell" aria-label={t.account}><div className="account-heading"><div><span className="console-kicker">{t.identity}</span><h1>{profile ? t.existingWelcome : registrationAvailable ? t.signInOrRegister : t.signIn}</h1></div>{profile ? <Action className="account-signout" onClick={signOut}><LogOut size={16} /> {t.signOut}</Action> : null}</div>{profile ? <div className="account-dashboard"><section className="account-profile-card">{profileAvatarUrl ? <img className="account-avatar" src={profileAvatarUrl} alt="" /> : null}<div><h2>{profile.user.display_name}</h2><p>{profile.user.username ?? profile.user.email}</p></div>{profile.proxy_base_url ? <div className="account-routing-address"><label>{t.trafficAddress}<Field readOnly value={profile.proxy_base_url} /></label><p className="account-help">{t.trafficAddressHint}</p><Action className="secondary-button" onClick={rotate}>{t.rotate}</Action></div> : null}<div className="identity-panel"><h3>{t.connectedMethods}</h3>{identities.map((identity) => <div className="identity-row" key={identity.id}><span><strong>{identity.provider_name}</strong><small>{identity.email ?? identity.provider_subject}</small></span><Action className="revoke-button" onClick={() => unlink(identity)}>{t.disconnect}</Action></div>)}{providers.some((provider) => !identities.some((identity) => identity.provider_slug === provider.slug)) ? <div className="provider-actions">{providers.filter((provider) => !identities.some((identity) => identity.provider_slug === provider.slug)).map((provider) => <a className="provider-button" href={providerUrl(provider, true)} key={provider.slug}>{t.connect} {provider.display_name}</a>)}</div> : null}</div><UserDownloads locale={locale} />{profile.user.username ? <UserPasswordChange locale={locale} onChanged={() => { setProfile(null); setUsage(null); setIdentities([]); setMessage(locale === 'zh' ? '密码已修改，请重新登录。' : 'Password changed. Please sign in again.') }} /> : null}<OperationToast locale={locale} notice={feedback ? { tone: feedbackSuccess ? 'success' : 'error', message: feedback } : null} /></section>{usage ? <section className="account-usage"><div className="console-section-head"><div><h2>{t.trafficUsage}</h2><p>{usage.month}{usage.group ? ` · ${usage.group.name}` : ''}</p></div></div><div className="console-metrics"><ConsoleMetric label={t.today} value={byteLabel(usage.today_response_bytes)} /><ConsoleMetric label={t.thisMonth} value={byteLabel(usage.response_bytes)} /><ConsoleMetric label={t.personalRemaining} value={usage.quota.remaining_bytes === null ? '∞' : byteLabel(usage.quota.remaining_bytes)} /><ConsoleMetric label={t.requests} value={usage.request_count.toLocaleString()} /></div>{usage.group ? <p className="account-help">{t.groupRemaining}: {usage.group.quota.remaining_bytes === null ? '∞' : byteLabel(usage.group.quota.remaining_bytes)}</p> : null}<div className="stats-columns"><div><h3>{t.byMirror}</h3>{usage.targets.map((target) => <div className="stat-row" key={target.target_code}><span>{target.target_code}</span><strong>{byteLabel(target.response_bytes)}</strong><small>{target.request_count} req</small></div>)}</div><div><h3>{t.recentTrend}</h3>{usage.daily.slice(-30).map((point) => <div className="stat-row" key={`${point.day}-${point.target_code}`}><span>{point.day.slice(5)} · {point.target_code}</span><strong>{byteLabel(point.response_bytes)}</strong><small>{point.error_count} err</small></div>)}</div></div></section> : null}</div> : <div className="account-auth-layout"><aside className={`registration-policy registration-policy-${policy?.tone ?? 'loading'}`}><span className="registration-policy-icon"><UserRound size={21} /></span><div><span className="eyebrow">REGISTRATION POLICY</span><h2>{policy?.title ?? '…'}</h2><p>{policy?.body}</p>{registration?.mode === 'domain_allowlist' && registration.allowed_email_domains.length ? <div className="allowed-domain-list"><small>{t.allowedDomains}</small>{registration.allowed_email_domains.map((domain) => <code key={domain}>@{domain}</code>)}</div> : null}</div></aside><div className="auth-methods"><PasswordAccess locale={locale} token={registrationToken} onSignedIn={async () => { await loadProfile(); setRegistrationToken(null) }} />{!registrationToken && providers.length ? <section className="auth-method-card"><div className="auth-method-head"><span><UserRound size={19} /></span><div><h2>{t.providerMethod}</h2><p>{t.providerMethodHint}</p></div></div><div className="provider-actions provider-actions-stacked">{providers.map((provider) => <a className="provider-button" href={providerUrl(provider)} key={provider.slug}><span><LogIn size={16} /> {provider.display_name}</span><small>{registrationAvailable ? t.canRegister : t.loginOnly}</small></a>)}</div></section> : null}{!registrationToken && registration?.email_login_enabled ? <section className="auth-method-card email-auth-card"><div className="auth-method-head"><span><Mail size={19} /></span><div><h2>{t.emailMethod}</h2><p>{t.emailMethodHint}</p></div></div><SettingsForm className="account-form" onSubmit={requestLogin}><label>{t.email}<Field required autoComplete="email" type="email" value={email} onChange={(event) => setEmail(event.target.value)} /></label><Action className="primary-button" disabled={sending} type="submit">{sending ? t.sending : t.sendCode}</Action></SettingsForm><div className="auth-divider"><span>{t.codeFallback}</span></div><SettingsForm className="account-form account-code-form" onSubmit={verify}><label>{t.code}<Field required autoComplete="one-time-code" inputMode="numeric" pattern="[0-9]{6}" value={code} onChange={(event) => setCode(event.target.value)} /></label><p>{t.codeHint}</p><Action className="primary-button" disabled={verifying} type="submit">{verifying ? t.verifying : t.verify}</Action></SettingsForm></section> : null}<OperationToast locale={locale} notice={feedback ? { tone: feedbackSuccess ? 'success' : 'error', message: feedback } : null} /></div></div>}</section></main>
}

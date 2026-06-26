// reason-map 的品牌标记:一杆"天平"——左实心节点 = 论点,右空心节点 = 反驳,
// 中间支点。呼应产品内核:把论证两端摊开称量,判定权在你手里(SPEC)。
// 用 currentColor,颜色由外层 color 决定(默认墨绿 --accent)。

export default function BrandLogo({ size = 26 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 32 32"
      fill="none"
      stroke="currentColor"
      aria-label="reason-map"
      role="img"
    >
      {/* 横梁:左节点中心连到右环左缘,不穿过环心 */}
      <line x1="10" y1="13" x2="18" y2="13" strokeWidth="1.7" strokeLinecap="round" />
      {/* 左:论点(实心) */}
      <circle cx="10" cy="13" r="4" fill="currentColor" stroke="none" />
      {/* 右:反驳(空心) */}
      <circle cx="22" cy="13" r="3.6" strokeWidth="1.8" />
      {/* 支柱 */}
      <line x1="16" y1="13" x2="16" y2="17.5" strokeWidth="1.9" strokeLinecap="round" />
      {/* 支点(三角底座) */}
      <path d="M16 17 L11.8 24 L20.2 24 Z" fill="currentColor" stroke="none" />
    </svg>
  );
}

import { check } from '@tauri-apps/plugin-updater';
import { h } from './ui/dom';

/** 有新版就在畫面最上方顯示橫幅；檢查失敗（例如沒網路）就安靜略過。 */
export async function showUpdateBanner(container: HTMLElement): Promise<void> {
  const update = await check().catch(() => null);
  if (!update) return;

  const text = h('span', {}, `有新版本 ${update.version} 可以更新。`);
  // 橫幅是琥珀色底，用一般（深色）按鍵才看得出來
  const button = h('button', {}, '立即更新');
  button.addEventListener('click', async () => {
    button.disabled = true;
    text.textContent = '正在下載更新，完成後會自動重新開啟…';
    try {
      // Windows 上下載完會直接啟動安裝程式並結束本程式，裝完由安裝程式重新開啟，所以不用 relaunch()
      await update.downloadAndInstall();
    } catch {
      text.textContent = '更新失敗，請稍後再試。';
      button.disabled = false;
    }
  });
  container.replaceChildren(h('div', { class: 'banner' }, text, button));
}

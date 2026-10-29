import { h } from './dom';

export interface ConfirmOptions {
  title: string;
  message: string;
  /** 安全的選項：琥珀色主鍵，一打開就選中，所以 Enter、空白鍵、Esc 都等於選它。 */
  stay: string;
  /** 要確認的動作。 */
  leave: string;
}

/**
 * 在畫面正中央跳出確認小視窗（黑色機身＋琥珀色螢幕窗），選了 `leave` 才回傳 true。
 * 用 <dialog> 的 showModal：背後的畫面點不到也按不到，點小視窗外面不會有反應，
 * 所以連點兩下的第二下不會誤按到裡面的鍵。
 */
export function confirmDialog(o: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    let answer = false;
    const choose = (leave: boolean) => {
      answer = leave;
      dialog.close();
    };
    const stayBtn = h('button', { class: 'primary big', onclick: () => choose(false) }, o.stay);
    const leaveBtn = h('button', { class: 'big', onclick: () => choose(true) }, o.leave);
    const dialog = h(
      'dialog',
      { class: 'confirm', 'aria-labelledby': 'confirm-title', 'aria-describedby': 'confirm-message' },
      h(
        'div',
        { class: 'display warn' },
        h('p', { class: 'display-title', id: 'confirm-title' }, o.title),
        h('p', { id: 'confirm-message' }, o.message),
      ),
      // 主鍵放右邊，和剪歌畫面下方的「回首頁／存成 MP3」同一個排法
      h('div', { class: 'actions' }, leaveBtn, stayBtn),
    );
    // 點到小視窗外面或文字（例如連點兩下的第二下）焦點會跑到小視窗本身，Enter 就沒作用了；拉回安全的鍵
    dialog.addEventListener('click', (e) => {
      if (!(e.target instanceof Element && e.target.closest('button'))) stayBtn.focus();
    });
    // Esc 會直接觸發 close，answer 還是 false
    dialog.addEventListener('close', () => {
      dialog.remove();
      resolve(answer);
    });
    document.body.append(dialog);
    dialog.showModal();
    stayBtn.focus();
  });
}

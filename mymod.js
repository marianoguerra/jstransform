import { a } from "mod1";
import { b } from "mod2";
import { c } from "app.js";
import * as util from "util.js";

function main() {
  const markup = html`
        <div
          @domain
          class="task-item"
          draggable="true"
          data-dragtype="todo-task"
          data-droptarget="todo-task"
          @on.drop="drop"
        >
          <input type="checkbox" :checked="$checked" @on.input="setCheck" />
          <input
            :value="$title"
            :disabled="$checked"
            @on.input="setTitle"
            class="form-control task-item-title"
          />
          <gd-btn
            type="danger"
            class="task-item-rm"
            icon="remove"
            @on.click="remove"
          ></gd-btn>
        </div>
  `,
    style = css`
        .task-item {
          display: grid;
          grid-template-columns: 1em 1fr auto;
          align-items: center;
          gap: var(--size-3);
          cursor: grab;
        }
        .task-item-title {
          border: none;
          padding: var(--size-2);
        }
        .task-item-rm {
          visibility: hidden;
        }
        .task-item:hover {
          outline: 2px solid var(--color-border);
          outline-offset: var(--size-2);
          position: relative;
          z-index: 100;
        }
        .task-item:hover > .task-item-rm {
          visibility: visible;
        }
        .task-item {
          transition: 0.15s border-top ease;
          border-top: 0 solid transparent;
        }
        .task-item[data-draggingover="todo-task"] {
          border-top: 4rem solid transparent;
          transition: 0.15s border-top ease;
        }
        .task-item[data-dragging] {
          opacity: var(--translucent);
        }
  `;

  console.log(
    a,
    b,
    c,
    util,
    css`with var ${a}`,
  );
}

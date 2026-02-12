// HRML Playground — example sources
export const examples = {
  counter: `state
  count: 0

div .counter
  button @click="count--" "-"
  span "{count}"
  button @click="count++" "+"`,

  toggle: `state
  visible: true

button @click="visible = !visible" "Toggle"
div :show="visible" "Content"`,

  input: `state
  name: ""

input :model="name" placeholder="Type your name"
span "Hello {name}!"`
};

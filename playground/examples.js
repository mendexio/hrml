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
span "Hello {name}!"`,

  form: `state
  name: ""
  email: ""
  submitted: false

form @submit.prevent="submitted = true"
  input :model="name" placeholder="Name" type="text"
  input :model="email" placeholder="Email" type="email"
  button type="submit" "Submit"

div :show="submitted"
  p "Thanks, {name}!"`,

  todo: `state
  task1: "Buy groceries"
  task2: "Write code"
  task3: "Read docs"
  done: false

div
  h2 "My Tasks"
  div
    input type="checkbox" :model="done"
    span " Mark all done"
  p "{task1}"
  p "{task2}"
  p "{task3}"
  p :show="done" "All tasks completed!"`,

  tabs: `state
  activeTab: "home"

div
  button @click="activeTab = 'home'" "Home"
  button @click="activeTab = 'profile'" "Profile"
  button @click="activeTab = 'about'" "About"

div :show="activeTab === 'home'"
  p "Welcome home!"

div :show="activeTab === 'profile'"
  p "Your profile"

div :show="activeTab === 'about'"
  p "About us"`,

  fetch: `state
  loading: false
  data: ""

div
  button @click="loading = true" "Load"
  button @click="loading = false" "Done"

div :show="loading"
  p "Loading..."

div :show="!loading"
  p "Ready"

// Server communication with $ is planned`
};

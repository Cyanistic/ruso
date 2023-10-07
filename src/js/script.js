document.addEventListener('DOMContentLoaded', () => {
  document.querySelectorAll('.triangle-container').forEach((container) => {
    console.log(container);
    container.addEventListener("mousemove", (e) =>{
      document.querySelectorAll(".triangle-up").forEach((shift) => {
        const position = window.getComputedStyle(shift).getPropertyValue("--speed");
        console.log(position);
        const x = (e.clientX * position) / 1000;
        const y = (e.clientY * position) / 2000;

        shift.style.transform = `translate(${x}px, ${y}px) scale(1)`;
      });
    });
  });
})

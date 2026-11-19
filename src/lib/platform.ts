// Nhận diện thiết bị cảm ứng/di động để đổi bố cục
export const isMobile = (): boolean =>
  /android|iphone|ipad|ipod/i.test(navigator.userAgent) ||
  (navigator.maxTouchPoints > 1 && window.innerWidth < 1100);
